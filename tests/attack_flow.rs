use parabellum::{
    app::commands::{AttackCommand, AttackCommandHandler},
    db::{establish_test_connection_pool, repository::PostgresRepository, test_factories::*},
    game::models::buildings::BuildingName,
    jobs::{worker::JobWorker, JobTask},
    repository::JobRepository,
};
use sqlx::{Executor, Postgres, Transaction};
use std::sync::Arc;

mod common;

// Definiamo un tipo alias per chiarezza
type TxRepo<'a> = PostgresRepository<Transaction<'a, Postgres>>;

#[tokio::test]
async fn test_full_attack_flow() {
    // 1. Setup della connessione e della transazione
    let pool = establish_test_connection_pool().await.unwrap();
    let mut tx = pool.begin().await.unwrap();

    // Ottieni un riferimento mutabile all'esecutore della transazione
    let tx_executor = tx.as_mut();

    // 2. ARRANGE: Crea le entità del test usando direttamente la transazione
    let attacker_player = player_factory(tx_executor, Default::default()).await;
    let attacker_village = village_factory(
        tx_executor,
        VillageFactoryOptions {
            player_id: Some(attacker_player.id),
            ..Default::default()
        },
    )
    .await;
    let attacker_army = army_factory(
        tx_executor,
        ArmyFactoryOptions {
            player_id: Some(attacker_player.id),
            village_id: Some(attacker_village.id),
            units: Some([100, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            tribe: Some(attacker_player.tribe),
            ..Default::default()
        },
    )
    .await;
    let defender_player = player_factory(tx_executor, Default::default()).await;
    let defender_village = village_factory(
        tx_executor,
        VillageFactoryOptions {
            player_id: Some(defender_player.id),
            ..Default::default()
        },
    )
    .await;

    // 3. ACT (Phase 1): Esegui il comando
    // Ora il repository contiene la transazione stessa, che può essere clonata
    let repo: Arc<TxRepo> = Arc::new(PostgresRepository::new(tx));

    let handler = AttackCommandHandler::new(repo.clone(), repo.clone());
    handler.handle(attack_command).await.unwrap();

    // 4. ASSERT (Phase 1): Controlla la creazione del job
    let jobs = repo.find_and_lock_due_jobs(10).await.unwrap();
    assert_eq!(
        jobs.len(),
        1,
        "There should be exactly one job in the queue."
    );
    let attack_job = &jobs[0];

    assert!(matches!(attack_job.task, JobTask::Attack(_)));
    assert_eq!(attack_job.status, parabellum::jobs::JobStatus::Processing);

    // 5. ACT (Phase 2): Esegui il worker
    let worker = Arc::new(JobWorker::new(repo.clone(), repo.clone(), repo.clone()));
    worker
        .process_jobs(&vec![attack_job.clone()])
        .await
        .unwrap();

    // 6. ASSERT (Phase 2): Controlla il risultato
    let final_jobs = repo.list_by_player_id(attacker_player.id).await.unwrap();
    assert_eq!(final_jobs.len(), 1);

    if let Ok(Some(original_job)) = repo.get_by_id(attack_job.id).await {
        assert_eq!(original_job.status, parabellum::jobs::JobStatus::Completed);
    }

    let return_job = &final_jobs[0];
    assert!(matches!(return_job.task, JobTask::ArmyReturn(_)));
    assert_eq!(return_job.status, parabellum::jobs::JobStatus::Pending);

    // 7. Cleanup: Il rollback viene gestito implicitamente quando `tx` esce dallo scope
}
