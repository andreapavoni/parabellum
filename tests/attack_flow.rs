use anyhow::Result;
use serial_test::serial;
use std::sync::Arc;

use parabellum::{
    app::commands::{AttackCommand, AttackCommandHandler},
    db::{establish_test_connection_pool, uow::PostgresUnitOfWorkProvider},
    game::{
        models::{buildings::BuildingName, map::Position, village::Village, ResourceGroup},
        test_factories::{
            army_factory, player_factory, valley_factory, village_factory, ArmyFactoryOptions,
            PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions,
        },
    },
    jobs::{worker::JobWorker, JobStatus, JobTask},
    repository::uow::UnitOfWorkProvider,
};

mod common;
use common::setup;

#[tokio::test]
#[serial]
async fn test_full_attack_flow() -> Result<()> {
    setup().await?;
    let pool = establish_test_connection_pool().await.unwrap();
    // 1. Crea il provider UoW invece del vecchio repo
    let uow_provider = Arc::new(PostgresUnitOfWorkProvider::new(pool));

    // --- 1. SETUP DATI (dentro una transazione) ---
    let (attacker_player, attacker_village, attacker_army, defender_village) = {
        let uow = uow_provider.begin().await?;
        let player_repo = uow.players();
        let village_repo = uow.villages();
        let army_repo = uow.armies();

        let attacker_player = player_factory(PlayerFactoryOptions::default());
        player_repo.create(&attacker_player).await?;

        let attacker_valley = valley_factory(ValleyFactoryOptions {
            position: Some(Position { x: 10, y: 10 }),
            ..Default::default()
        });
        // I village_factory ora creano modelli di dominio, che passiamo al repo
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
            tribe: Some(attacker_player.tribe.clone()),
            ..Default::default()
        });
        army_repo.create(&attacker_army).await?;

        let defender_player = player_factory(PlayerFactoryOptions::default());
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

        uow.commit().await?;

        // Ritorna i dati necessari per il test
        (
            attacker_player,
            attacker_village,
            attacker_army,
            defender_village,
        )
    };

    // --- 2. ACT (Phase 1): Esegui il comando di attacco ---
    let attack_command = AttackCommand {
        player_id: attacker_player.id,
        village_id: attacker_village.id,
        army_id: attacker_army.id,
        target_village_id: defender_village.id,
        catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
    };

    // Il comando viene eseguito in una nuova transazione
    {
        let uow_attack = uow_provider.begin().await?;
        let handler = AttackCommandHandler::new(
            uow_attack.jobs(),
            uow_attack.villages(),
            uow_attack.armies(),
        );
        handler.handle(attack_command).await.unwrap();
        uow_attack.commit().await?;
    };

    // --- 3. ASSERT (Phase 1): Controlla che il job sia stato creato ---
    let attack_job = {
        let uow_assert1 = uow_provider.begin().await?;
        let job_repo = uow_assert1.jobs();
        let jobs = job_repo
            .list_by_player_id(attacker_player.id)
            .await
            .unwrap();

        assert_eq!(jobs.len(), 1, "There should be exactly 1 job in the queue.");
        let attack_job = &jobs[0];

        assert!(matches!(attack_job.task, JobTask::Attack(_)));
        assert_eq!(
            attack_job.status,
            JobStatus::Pending,
            "Expected job status set to Pending, got {:?}",
            attack_job.status
        );
        uow_assert1.rollback().await?; // Solo lettura, rollback
        attack_job.clone() // Clona il job per usarlo dopo
    };

    // --- 4. ACT (Phase 2): Simula il worker che processa il job ---
    let worker = Arc::new(JobWorker::new(uow_provider.clone()));
    worker
        .process_jobs(&vec![attack_job.clone()])
        .await
        .unwrap();

    // --- 5. ASSERT (Phase 2): Controlla l'esito del job ---
    {
        let uow_assert2 = uow_provider.begin().await?;
        let job_repo = uow_assert2.jobs();
        let final_jobs = job_repo
            .list_by_player_id(attacker_player.id)
            .await
            .unwrap();

        // Ci aspettiamo 1 solo job PENDING (il ritorno)
        assert_eq!(
            final_jobs.len(),
            1,
            "There should be exactly 1 pending job in the queue (the return)."
        );

        // Controlla che il job originale sia 'Completed'
        if let Ok(Some(original_job)) = job_repo.get_by_id(attack_job.id).await {
            assert_eq!(original_job.status, JobStatus::Completed);
        }

        let return_job = &final_jobs[0];
        assert!(
            matches!(return_job.task, JobTask::ArmyReturn(_)),
            "Expected an army return job task, got: {:?}",
            return_job
        );
        assert_eq!(return_job.status, JobStatus::Pending);

        uow_assert2.rollback().await?;
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_attack_with_catapult_damage_and_bounty() -> Result<()> {
    setup().await?;

    // --- 1. SETUP ---
    let pool = establish_test_connection_pool().await?;
    let uow_provider = Arc::new(PostgresUnitOfWorkProvider::new(pool));

    // Dati creati in una transazione
    let (
        attacker_player,
        attacker_village,
        attacker_army,
        defender_village,
        initial_warehouse_level,
        initial_defender_resources,
    ) = {
        let uow = uow_provider.begin().await?;
        let player_repo = uow.players();
        let village_repo = uow.villages();
        let army_repo = uow.armies();

        let attacker_player = player_factory(PlayerFactoryOptions::default());
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
            units: Some([0, 0, 100, 0, 0, 0, 0, 100, 0, 0]),
            ..Default::default()
        });

        let defender_player = player_factory(PlayerFactoryOptions::default());
        let defender_valley = valley_factory(ValleyFactoryOptions {
            position: Some(Position { x: 40, y: 40 }),
            ..Default::default()
        });
        let mut defender_village = village_factory(VillageFactoryOptions {
            valley: Some(defender_valley),
            player: Some(defender_player.clone()),
            ..Default::default()
        });
        // Modifica l'oggetto di dominio
        defender_village.add_building(BuildingName::Granary, 21)?;
        defender_village.add_building(BuildingName::Warehouse, 20)?;
        defender_village
            .stocks
            .store_resources(ResourceGroup::new(800, 800, 800, 800));
        // Aggiorna lo stato per ricalcolare la popolazione, etc.
        defender_village.update_state();

        let defender_army = army_factory(ArmyFactoryOptions {
            village_id: Some(defender_village.id),
            player_id: Some(defender_player.id),
            tribe: Some(defender_player.tribe.clone()),
            units: Some([5, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            ..Default::default()
        });

        // Salva tutto
        player_repo.create(&attacker_player).await?;
        player_repo.create(&defender_player).await?;
        village_repo.create(&attacker_village).await?;
        village_repo.create(&defender_village).await?; // Crea il villaggio base
        village_repo.save(&defender_village).await?; // Salva le modifiche (edifici, stock)
        army_repo.create(&attacker_army).await?;
        army_repo.create(&defender_army).await?;

        let initial_warehouse_level = defender_village
            .get_building_by_name(BuildingName::Warehouse)
            .unwrap()
            .building
            .level;
        let initial_defender_resources = defender_village.stocks.stored_resources().total();

        uow.commit().await?;

        (
            attacker_player,
            attacker_village,
            attacker_army,
            defender_village,
            initial_warehouse_level,
            initial_defender_resources,
        )
    };

    // --- 2. ACT (Phase 1): Esegui comando attacco ---
    let attack_command = AttackCommand {
        player_id: attacker_player.id,
        village_id: attacker_village.id,
        army_id: attacker_army.id,
        target_village_id: defender_village.id,
        catapult_targets: [BuildingName::Warehouse, BuildingName::Granary],
    };

    {
        let uow_attack = uow_provider.begin().await?;
        let handler = AttackCommandHandler::new(
            uow_attack.jobs(),
            uow_attack.villages(),
            uow_attack.armies(),
        );
        handler.handle(attack_command).await.unwrap();
        uow_attack.commit().await?;
    }

    // --- 3. ACT (Phase 2): Processa il job ---
    let jobs = {
        let uow_read_jobs = uow_provider.begin().await?;
        let jobs = uow_read_jobs
            .jobs()
            .list_by_player_id(attacker_player.id)
            .await
            .unwrap();
        uow_read_jobs.rollback().await?;
        jobs
    };

    let worker = Arc::new(JobWorker::new(uow_provider.clone()));
    worker.process_jobs(&jobs).await.unwrap();

    // --- 4. ASSERT ---
    // Ricarica i dati dal db in una nuova transazione per verificare
    {
        let uow_assert = uow_provider.begin().await?;
        let village_repo = uow_assert.villages();
        let job_repo = uow_assert.jobs();

        let updated_defender_village: Village =
            village_repo.get_by_id(defender_village.id).await?.unwrap();

        let return_job = job_repo
            .list_by_player_id(attacker_player.id)
            .await?
            .pop()
            .unwrap();

        // Danni agli edifici
        let final_warehouse_level = updated_defender_village
            .get_building_by_name(BuildingName::Warehouse)
            .map_or(0, |b| b.building.level);

        assert!(
            final_warehouse_level < initial_warehouse_level,
            "Warehouse should have been damaged (Before: {}, After: {}).",
            initial_warehouse_level,
            final_warehouse_level
        );

        // Bottino (Bounty)
        if let JobTask::ArmyReturn(return_payload) = return_job.task {
            let bounty = return_payload.resources.total();
            assert!(bounty > 0, "Attacker should have stolen resources.");

            let final_defender_resources =
                updated_defender_village.stocks.stored_resources().total();
            assert!(
                final_defender_resources < initial_defender_resources,
                "Defender resources should have been reduced (Before: {}, After: {}).",
                initial_defender_resources,
                final_defender_resources
            );
        } else {
            panic!("Army return job isn't correct.");
        }

        uow_assert.rollback().await?;
    }

    Ok(())
}
