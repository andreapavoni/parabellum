use parabellum::{
    Result,
    app::{
        commands::{AttackVillage, AttackVillageHandler},
        job_registry::AppJobRegistry,
    },
    cqrs::CommandHandler,
    db::establish_test_connection_pool,
    game::{
        models::{
            Player, ResourceGroup, Tribe,
            army::{Army, TroopSet},
            buildings::BuildingName,
            map::Position,
            village::Village,
        },
        test_factories::{
            ArmyFactoryOptions, PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions,
            army_factory, player_factory, valley_factory, village_factory,
        },
    },
    jobs::{
        JobStatus,
        tasks::{ArmyReturnTask, AttackTask},
        worker::JobWorker,
    },
    repository::uow::UnitOfWorkProvider,
};
use serial_test::serial;
use std::sync::Arc;
use tokio::sync::Mutex;

mod test_utils;
use test_utils::TestUnitOfWorkProvider;

#[tokio::test]
#[serial]
async fn test_full_attack_flow() -> Result<()> {
    let pool = establish_test_connection_pool().await.unwrap();
    let master_tx = pool.begin().await.unwrap();
    let master_tx_arc = Arc::new(Mutex::new(master_tx));
    let app_registry = Arc::new(AppJobRegistry::new());

    let uow_provider: Arc<dyn UnitOfWorkProvider> =
        Arc::new(TestUnitOfWorkProvider::new(master_tx_arc.clone()));

    let (attacker_player, attacker_village, attacker_army) = {
        setup_player_party(
            uow_provider.clone(),
            Position { x: 10, y: 10 },
            Tribe::Roman,
            [100, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        )
        .await?
    };

    let (_defender_player, defender_village, _defender_army) = {
        setup_player_party(
            uow_provider.clone(),
            Position { x: 20, y: 20 },
            Tribe::Roman,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        )
        .await?
    };

    let attack_command = AttackVillage {
        player_id: attacker_player.id,
        village_id: attacker_village.id,
        army_id: attacker_army.id,
        target_village_id: defender_village.id,
        catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
    };

    {
        let uow_attack = uow_provider.begin().await?;
        {
            let handler = AttackVillageHandler::new();
            handler.handle(attack_command, &uow_attack).await.unwrap();
        }

        uow_attack.commit().await?;
    };

    let attack_job = {
        let uow_assert1 = uow_provider.begin().await?;
        let cloned_job;

        {
            let job_repo = uow_assert1.jobs();
            let jobs = job_repo
                .list_by_player_id(attacker_player.id)
                .await
                .unwrap();

            assert_eq!(jobs.len(), 1, "There should be exactly 1 job in the queue.");
            let attack_job = &jobs[0];

            assert_eq!(attack_job.task.task_type, "Attack");
            let task_data: Result<AttackTask, _> =
                serde_json::from_value(attack_job.task.data.clone());
            assert!(task_data.is_ok(), "Job data should be a valid AttackTask");
            assert_eq!(task_data.unwrap().army_id, attacker_army.id);

            assert_eq!(
                attack_job.status,
                JobStatus::Pending,
                "Expected job status set to Pending, got {:?}",
                attack_job.status
            );

            cloned_job = attack_job.clone();
        }

        uow_assert1.rollback().await?;
        cloned_job
    };

    // --- 4. ACT (Phase 2): Simulate worker processing job ---
    let worker = Arc::new(JobWorker::new(uow_provider.clone(), app_registry));
    worker
        .process_jobs(&vec![attack_job.clone()])
        .await
        .unwrap();

    // --- 5. ASSERT (Phase 2): Check job result ---
    {
        let uow_assert2 = uow_provider.begin().await?;
        let job_repo = uow_assert2.jobs();
        let final_jobs = job_repo
            .list_by_player_id(attacker_player.id)
            .await
            .unwrap();

        assert_eq!(
            final_jobs.len(),
            1,
            "There should be exactly 1 pending job in the queue (the return)."
        );

        if let Ok(original_job) = job_repo.get_by_id(attack_job.id).await {
            assert_eq!(original_job.status, JobStatus::Completed);
        }

        let return_job = &final_jobs[0];
        assert_eq!(return_job.task.task_type, "ArmyReturn");
        let return_task_data: Result<ArmyReturnTask, _> =
            serde_json::from_value(return_job.task.data.clone());
        assert!(
            return_task_data.is_ok(),
            "Job data should be a valid ArmyReturnTask"
        );
        assert_eq!(return_job.status, JobStatus::Pending);

        uow_assert2.rollback().await?;
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_attack_with_catapult_damage_and_bounty() -> Result<()> {
    // setup().await?;

    // --- 1. SETUP ---
    let pool = establish_test_connection_pool().await.unwrap();
    let master_tx = pool.begin().await.unwrap();
    let master_tx_arc = Arc::new(Mutex::new(master_tx));
    let app_registry = Arc::new(AppJobRegistry::new());

    let uow_provider: Arc<dyn UnitOfWorkProvider> =
        Arc::new(TestUnitOfWorkProvider::new(master_tx_arc.clone()));

    let attacker_player: Player;
    let attacker_village: Village;
    let attacker_army: Army;
    let mut defender_village: Village;

    (attacker_player, attacker_village, attacker_army) = {
        setup_player_party(
            uow_provider.clone(),
            Position { x: 10, y: 10 },
            Tribe::Roman,
            [100, 0, 0, 0, 0, 0, 0, 100, 0, 0],
        )
        .await?
    };

    (_, defender_village, _) = {
        setup_player_party(
            uow_provider.clone(),
            Position { x: 20, y: 20 },
            Tribe::Teuton,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        )
        .await?
    };

    {
        let uow_update = uow_provider.begin().await?;

        {
            let village_repo = uow_update.villages();
            defender_village.add_building(BuildingName::Granary, 21)?;
            defender_village.add_building(BuildingName::Warehouse, 20)?;
            defender_village
                .stocks
                .store_resources(ResourceGroup::new(800, 800, 800, 800));
            defender_village.update_state();

            village_repo.save(&defender_village).await?;
        }

        uow_update.commit().await?;
    };

    let initial_warehouse_level = defender_village
        .get_building_by_name(BuildingName::Warehouse)
        .unwrap()
        .building
        .level;

    let initial_defender_resources = defender_village.stocks.stored_resources().total();

    // --- 2. ACT (Phase 1): Execute attack command ---
    let attack_command = AttackVillage {
        player_id: attacker_player.id,
        village_id: attacker_village.id,
        army_id: attacker_army.id,
        target_village_id: defender_village.id,
        catapult_targets: [BuildingName::Warehouse, BuildingName::Granary],
    };

    {
        let uow_attack = uow_provider.begin().await?;

        {
            let handler = AttackVillageHandler::new();
            handler.handle(attack_command, &uow_attack).await.unwrap();
        }

        uow_attack.commit().await?; // OK
    };

    // --- 3. ACT (Phase 2): Process job ---
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

    let worker = Arc::new(JobWorker::new(uow_provider.clone(), app_registry));
    worker.process_jobs(&jobs).await.unwrap();

    // --- 4. ASSERT ---
    {
        let uow_assert = uow_provider.begin().await?;
        let village_repo = uow_assert.villages();
        let job_repo = uow_assert.jobs();

        let updated_defender_village: Village = village_repo.get_by_id(defender_village.id).await?;

        let return_jobs = job_repo.list_by_player_id(attacker_player.id).await?;
        assert_eq!(
            return_jobs.len(),
            1,
            "There should be exactly 1 return job after the attack."
        );
        let return_job = return_jobs.first().unwrap();

        // Building damages
        let final_warehouse_level = updated_defender_village
            .get_building_by_name(BuildingName::Warehouse)
            .map_or(0, |b| b.building.level);

        assert!(
            final_warehouse_level < initial_warehouse_level,
            "Warehouse should have been damaged (Before: {}, After: {}).",
            initial_warehouse_level,
            final_warehouse_level
        );

        // Bounty

        assert_eq!(return_job.task.task_type, "ArmyReturn".to_string());
        let return_payload: ArmyReturnTask = serde_json::from_value(return_job.task.data.clone())?;

        let bounty = return_payload.resources.total();
        assert!(bounty > 0, "Attacker should have stolen resources.");

        let final_defender_resources = updated_defender_village.stocks.stored_resources().total();
        assert!(
            final_defender_resources < initial_defender_resources,
            "Defender resources should have been reduced (Before: {}, After: {}).",
            initial_defender_resources,
            final_defender_resources
        );

        uow_assert.rollback().await?;
    }

    Ok(())
}

async fn setup_player_party(
    uow_provider: Arc<dyn UnitOfWorkProvider>,
    position: Position,
    tribe: Tribe,
    units: TroopSet,
) -> Result<(Player, Village, Army)> {
    let uow = uow_provider.begin().await?;
    let player: Player;
    let village: Village;
    let army: Army;
    {
        let player_repo = uow.players();
        let village_repo = uow.villages();
        let army_repo = uow.armies();

        player = player_factory(PlayerFactoryOptions {
            tribe: Some(tribe.clone()),
            ..Default::default()
        });
        player_repo.create(&player).await?;

        let valley = valley_factory(ValleyFactoryOptions {
            position: Some(position),
            ..Default::default()
        });
        village = village_factory(VillageFactoryOptions {
            valley: Some(valley),
            player: Some(player.clone()),
            ..Default::default()
        });
        village_repo.create(&village).await?;

        army = army_factory(ArmyFactoryOptions {
            player_id: Some(player.id),
            village_id: Some(village.id),
            units: Some(units),
            tribe: Some(tribe.clone()),
            ..Default::default()
        });
        army_repo.create(&army).await?;
    }

    uow.commit().await?;

    Ok((player, village, army))
}
