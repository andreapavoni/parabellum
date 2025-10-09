use parabellum::{
    app::commands::{AttackCommand, AttackCommandHandler}, // We will use the handler directly
    db::test_factories::*,
    game::models::buildings::BuildingName,
    jobs::{worker::JobWorker, JobTask},
    repository::JobRepository,
};
use std::sync::Arc;

// Import the setup helper
mod common;

#[tokio::test]
async fn test_full_attack_flow() {
    // The test pool is created here and a transaction is started.
    let (_app, repo, pool) = common::setup_test_env().await;
    let mut conn = pool.get().await.unwrap();

    // 1. ARRANGE: Set up the world state using your DB test factories.
    let attacker_player = player_factory(&mut conn, Default::default()).await;
    let attacker_village = village_factory(
        &mut conn,
        parabellum::db::test_factories::VillageFactoryOptions {
            player_id: Some(attacker_player.id),
            ..Default::default()
        },
    )
    .await;
    let attacker_army = army_factory(
        &mut conn,
        ArmyFactoryOptions {
            player_id: Some(attacker_player.id),
            village_id: Some(attacker_village.id),
            units: Some([100, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            tribe: Some(attacker_player.tribe),
            ..Default::default()
        },
    )
    .await;

    let defender_player = player_factory(&mut conn, Default::default()).await;
    let defender_village = village_factory(
        &mut conn,
        parabellum::db::test_factories::VillageFactoryOptions {
            player_id: Some(defender_player.id),
            ..Default::default()
        },
    )
    .await;

    // 2. ACT (Phase 1): A user sends an attack command.
    let attack_command = AttackCommand {
        player_id: attacker_player.id,
        village_id: attacker_village.id as u32,
        army_id: attacker_army.id,
        target_village_id: defender_village.id as u32,
        catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
    };

    // We instantiate and run the handler directly for the test.
    let handler = AttackCommandHandler::new(repo.clone(), repo.clone());
    handler.handle(attack_command).await.unwrap();

    // 3. ASSERT (Phase 1): Check if the attack job was created.
    let jobs = repo.find_and_lock_due_jobs(10).await.unwrap();
    assert_eq!(
        jobs.len(),
        1,
        "There should be exactly one job in the queue."
    );
    let attack_job = &jobs[0];

    assert!(matches!(attack_job.task, JobTask::Attack(_)));
    assert_eq!(
        attack_job.status,
        parabellum::jobs::JobStatus::Processing,
        "Expected job status set to processing, got {:?}",
        attack_job.status
    );

    // 4. ACT (Phase 2): Simulate time passing and run the worker.
    // Instantiate the worker with the test repositories.
    let worker = Arc::new(JobWorker::new(repo.clone(), repo.clone(), repo.clone()));

    // Run the worker's processing logic just once.
    worker
        .process_jobs(&vec![attack_job.clone()])
        .await
        .unwrap();

    println!(
        "-----------> player {} | job player {} | village player {}",
        attacker_player.id, attack_job.player_id, attacker_village.player_id
    );

    // 5. ASSERT (Phase 2): Check the outcome of the job processing.
    let final_jobs = repo.list_by_player_id(attacker_player.id).await.unwrap();

    assert_eq!(
        final_jobs.len(),
        1,
        "There should be exactly 1 job in the queue."
    );

    // Find the original job and assert it's completed.
    if let Ok(Some(original_job)) = repo.get_by_id(attack_job.id).await {
        assert_eq!(original_job.status, parabellum::jobs::JobStatus::Completed);
    }

    // Find the new job and assert it's the return trip.
    let return_job = &final_jobs[0];
    assert!(
        matches!(return_job.task, JobTask::ArmyReturn(_)),
        "Expected an army return job task, got: {:?}",
        return_job
    );
    assert_eq!(return_job.status, parabellum::jobs::JobStatus::Pending);

    // The transaction will be rolled back automatically here.
}
