use anyhow::Result;
use std::sync::Arc;

use parabellum::{
    app::commands::{AttackCommand, AttackCommandHandler},
    db::{establish_test_connection_pool, repository::*}, // Import the helper
    game::{
        models::{buildings::BuildingName, map::Position, village::Village},
        test_factories::*,
    },
    jobs::{worker::JobWorker, JobTask},
    repository::*,
};

#[tokio::test]
async fn test_full_attack_flow() -> Result<()> {
    let pool = establish_test_connection_pool().await.unwrap();
    let repo = Arc::new(PostgresRepository::new(pool));

    let army_repo: Arc<dyn ArmyRepository> = repo.clone();
    let job_repo: Arc<dyn JobRepository> = repo.clone();
    let player_repo: Arc<dyn PlayerRepository> = repo.clone();
    let village_repo: Arc<dyn VillageRepository> = repo.clone();

    let attacker_player = player_factory(Default::default());
    player_repo.create(&attacker_player).await?;

    let attacker_valley = valley_factory(ValleyFactoryOptions {
        position: Some(Position { x: 10, y: 10 }),
        ..Default::default()
    });

    let attacker_village = village_factory(VillageFactoryOptions {
        valley: Some(attacker_valley),
        player: Some(attacker_player.clone()),
        ..Default::default()
    });
    village_repo.create(&attacker_village).await?;

    let attacker_army = army_factory(ArmyFactoryOptions {
        player_id: Some(attacker_player.id),
        village_id: Some(attacker_village.id),
        units: Some([100, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        tribe: Some(attacker_player.tribe),
        ..Default::default()
    });
    army_repo.create(&attacker_army).await?;

    let defender_player = player_factory(Default::default());
    player_repo.create(&defender_player).await?;

    let defender_valley = valley_factory(ValleyFactoryOptions {
        position: Some(Position { x: 20, y: 20 }),
        ..Default::default()
    });

    let defender_village = village_factory(VillageFactoryOptions {
        player: Some(defender_player),
        valley: Some(defender_valley),
        ..Default::default()
    });
    village_repo.create(&defender_village).await?;

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
    let final_jobs = job_repo
        .list_by_player_id(attacker_player.id)
        .await
        .unwrap();

    assert_eq!(
        final_jobs.len(),
        1,
        "There should be exactly 1 job in the queue."
    );

    if let Ok(Some(original_job)) = job_repo.get_by_id(attack_job.id).await {
        assert_eq!(original_job.status, parabellum::jobs::JobStatus::Completed);
    }

    let return_job = &final_jobs[0];
    assert!(
        matches!(return_job.task, JobTask::ArmyReturn(_)),
        "Expected an army return job task, got: {:?}",
        return_job
    );
    assert_eq!(return_job.status, parabellum::jobs::JobStatus::Pending);

    Ok(())
}

#[tokio::test]
async fn test_attack_with_catapult_damage_and_bounty() -> Result<()> {
    // --- 1. SETUP ---
    let pool = establish_test_connection_pool().await.unwrap();
    let repo = Arc::new(PostgresRepository::new(pool));

    let player_repo: Arc<dyn PlayerRepository> = repo.clone();
    let village_repo: Arc<dyn VillageRepository> = repo.clone();
    let job_repo: Arc<dyn JobRepository> = repo.clone();

    // CREA MODELLI DI DOMINIO con le factories di dominio
    let attacker_player = player_factory(Default::default());
    let attacker_valley = valley_factory(ValleyFactoryOptions {
        position: Some(Position { x: 30, y: 30 }),
        ..Default::default()
    });

    let attacker_village = village_factory(VillageFactoryOptions {
        valley: Some(attacker_valley),
        player: Some(attacker_player.clone()),
        ..Default::default()
    });

    let attacker_army = army_factory(ArmyFactoryOptions {
        village_id: Some(attacker_village.id),
        player_id: Some(attacker_player.id),
        tribe: Some(attacker_player.tribe.clone()),
        units: None,
        smithy: None,
        hero: None,
    });

    let defender_player = player_factory(Default::default());

    let defender_valley = valley_factory(ValleyFactoryOptions {
        position: Some(Position { x: 40, y: 40 }),
        ..Default::default()
    });

    let mut defender_village = village_factory(VillageFactoryOptions {
        valley: Some(defender_valley),
        player: Some(defender_player.clone()),
        ..Default::default()
    });
    // Aggiungi un edificio di test direttamente sul modello di dominio
    defender_village.add_building(BuildingName::MainBuilding, 9)?;
    defender_village.add_building(BuildingName::Warehouse, 20)?;

    // SALVA I MODELLI DI DOMINIO nel DB per il test
    // (Questo richiederà di implementare la conversione inversa per il salvataggio)
    player_repo.create(&attacker_player).await?;
    player_repo.create(&defender_player).await?;
    village_repo.create(&attacker_village).await?;
    village_repo.create(&defender_village).await?;
    // ... e così via per gli eserciti etc.
    //
    // Aggiungiamo un edificio da colpire, ad esempio il Magazzino a livello 5
    defender_village
        .add_building(BuildingName::Warehouse, 20)
        .unwrap();
    repo.save(&defender_village).await?;

    let initial_warehouse_level = defender_village
        .get_building_by_name(BuildingName::Warehouse)
        .unwrap()
        .level;
    let initial_defender_resources = defender_village.stocks.total();

    // --- 2. ACT (Phase 1): Eseguire il comando di attacco ---
    let attack_command = AttackCommand {
        player_id: attacker_player.id,
        village_id: attacker_village.id as u32,
        army_id: attacker_army.id,
        target_village_id: defender_village.id as u32,
        catapult_targets: [BuildingName::Warehouse, BuildingName::Granary], // Bersagli
    };

    let handler = AttackCommandHandler::new(repo.clone(), repo.clone());
    handler.handle(attack_command).await.unwrap();

    // --- 3. ACT (Phase 2): Processare il job di attacco ---
    let jobs = repo.find_and_lock_due_jobs(10).await.unwrap();
    let worker = Arc::new(JobWorker::new(repo.clone(), repo.clone(), repo.clone()));
    worker.process_jobs(&jobs).await.unwrap();

    // --- 4. ASSERT ---
    // Ricarichiamo i dati dal DB per verificare gli effetti
    // let updated_defender_village = repo.get_by_id(defender_village.id as u32).await?.unwrap();
    let updated_defender_village: Village = village_repo
        .get_by_id(defender_village.id as u32)
        .await?
        .unwrap();
    let return_job = job_repo
        .list_by_player_id(attacker_player.id)
        .await?
        .pop()
        .unwrap();

    // Asserzione 1: Danno all'edificio
    let final_warehouse_level = updated_defender_village
        .get_building_by_name(BuildingName::Warehouse)
        .unwrap()
        .level;
    assert!(
        final_warehouse_level < initial_warehouse_level,
        "Il magazzino avrebbe dovuto subire danni."
    );

    // Asserzione 2: Bottino
    if let JobTask::ArmyReturn(return_payload) = return_job.task {
        let bounty = return_payload.resources.total();
        assert!(
            bounty > 0,
            "L'attaccante avrebbe dovuto saccheggiare delle risorse."
        );

        // Asserzione 3: Risorse del difensore diminuite
        let final_defender_resources = updated_defender_village.stocks.total();
        assert!(
            final_defender_resources < initial_defender_resources,
            "Le scorte del difensore sarebbero dovute diminuire."
        );
    } else {
        panic!("Il job di ritorno non è corretto.");
    }

    Ok(())
}
