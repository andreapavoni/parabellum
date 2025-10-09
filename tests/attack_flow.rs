use parabellum::{
    app::commands::{AttackCommand, AttackCommandHandler},
    db::{run_test_with_transaction, test_factories::*}, // Import the helper
    game::models::buildings::BuildingName,
    jobs::{worker::JobWorker, JobTask},
    repository::JobRepository,
};
use std::sync::Arc;

mod common;

#[tokio::test]
async fn test_full_attack_flow() {
    // Use the run_test_with_transaction helper
    run_test_with_transaction(|tx, repo| {
        Box::pin(async move {
            // 1. ARRANGE: All factories now use `tx`
            let attacker_player = player_factory(tx, Default::default()).await;
            let attacker_village = village_factory(
                tx,
                parabellum::db::test_factories::VillageFactoryOptions {
                    player_id: Some(attacker_player.id),
                    ..Default::default()
                },
            )
            .await;
            let attacker_army = army_factory(
                tx,
                ArmyFactoryOptions {
                    player_id: Some(attacker_player.id),
                    village_id: Some(attacker_village.id),
                    units: Some([100, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    tribe: Some(attacker_player.tribe),
                    ..Default::default()
                },
            )
            .await;

            let defender_player = player_factory(tx, Default::default()).await;
            let defender_village = village_factory(
                tx,
                parabellum::db::test_factories::VillageFactoryOptions {
                    player_id: Some(defender_player.id),
                    ..Default::default()
                },
            )
            .await;

            // 2. ACT (Phase 1): The handler now uses the same `repo`
            let attack_command = AttackCommand {
                player_id: attacker_player.id,
                village_id: attacker_village.id as u32,
                army_id: attacker_army.id,
                target_village_id: defender_village.id as u32,
                catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
            };

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
            let worker = Arc::new(JobWorker::new(repo.clone(), repo.clone(), repo.clone()));
            worker
                .process_jobs(&vec![attack_job.clone()])
                .await
                .unwrap();

            // 5. ASSERT (Phase 2): Check the outcome of the job processing.
            let final_jobs = repo.list_by_player_id(attacker_player.id).await.unwrap();

            assert_eq!(
                final_jobs.len(),
                1,
                "There should be exactly 1 job in the queue."
            );

            if let Ok(Some(original_job)) = repo.get_by_id(attack_job.id).await {
                assert_eq!(original_job.status, parabellum::jobs::JobStatus::Completed);
            }

            let return_job = &final_jobs[0];
            assert!(
                matches!(return_job.task, JobTask::ArmyReturn(_)),
                "Expected an army return job task, got: {:?}",
                return_job
            );
            assert_eq!(return_job.status, parabellum::jobs::JobStatus::Pending);

            Ok(()) // Signal success for the test
        })
    })
    .await;
}
