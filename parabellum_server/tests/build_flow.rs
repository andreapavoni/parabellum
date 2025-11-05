mod test_utils;

use parabellum_app::{
    command_handlers::AddBuildingCommandHandler,
    config::Config,
    cqrs::{CommandHandler, commands::AddBuilding},
    job_registry::AppJobRegistry,
    jobs::{JobStatus, tasks::AddBuildingTask, worker::JobWorker},
    uow::UnitOfWorkProvider,
};
use parabellum_core::Result;
use parabellum_db::establish_test_connection_pool;
use parabellum_game::{
    models::buildings::Building,
    test_utils::{
        PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions, player_factory,
        valley_factory, village_factory,
    },
};
use parabellum_types::{
    buildings::BuildingName, common::ResourceGroup, map::Position, tribe::Tribe,
};

use std::sync::Arc;
use test_utils::tests::TestUnitOfWorkProvider;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_full_build_flow() -> Result<()> {
    let pool = establish_test_connection_pool().await.unwrap();
    let master_tx = pool.begin().await.unwrap();
    let master_tx_arc = Arc::new(Mutex::new(master_tx));
    let app_registry = Arc::new(AppJobRegistry::new());
    let config = Arc::new(Config::from_env());

    let uow_provider: Arc<dyn UnitOfWorkProvider> =
        Arc::new(TestUnitOfWorkProvider::new(master_tx_arc.clone()));

    let (player, village) = {
        let uow = uow_provider.begin().await?;
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        uow.players().save(&player).await?;

        let valley = valley_factory(ValleyFactoryOptions {
            position: Some(Position { x: 1, y: 1 }),
            ..Default::default()
        });
        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            valley: Some(valley),
            ..Default::default()
        });

        village.set_building_level_at_slot(19, 3)?;
        let rally_point = Building::new(BuildingName::RallyPoint).at_level(1)?;
        village.add_building_at_slot(rally_point, 39).unwrap();

        village
            .stocks
            .store_resources(ResourceGroup(1000, 1000, 1000, 1000));
        village.update_state();
        uow.villages().save(&village).await?;

        uow.commit().await?;
        (player, village)
    };

    let cost = Building::new(BuildingName::Barracks).cost();
    let initial_lumber = village.stocks.lumber;
    let slot_to_build: u8 = 22;

    let command = AddBuilding {
        player_id: player.id,
        village_id: village.id,
        slot_id: slot_to_build,
        name: BuildingName::Barracks,
    };

    {
        let uow_command = uow_provider.begin().await?;
        let handler = AddBuildingCommandHandler::new();
        handler.handle(command, &uow_command, &config).await?;
        uow_command.commit().await?;
    }

    // --- 3. ASSERT (Phase 1): Check DB state after command ---
    let (job_to_run, village_id) = {
        let uow_assert1 = uow_provider.begin().await?;

        // Check village
        let updated_village = uow_assert1.villages().get_by_id(village.id).await?;
        assert_eq!(
            updated_village.stocks.lumber,
            initial_lumber - cost.resources.0,
            "Resources should be deducted"
        );
        assert!(
            updated_village
                .get_building_by_slot_id(slot_to_build)
                .is_none(),
            "Building should NOT exist yet"
        );

        // Check job
        let jobs = uow_assert1.jobs().list_by_player_id(player.id).await?;
        assert_eq!(jobs.len(), 1, "There should be exactly 1 job");
        let job = jobs.first().unwrap();
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.task.task_type, "AddBuilding");

        let task: AddBuildingTask = serde_json::from_value(job.task.data.clone())?;
        assert_eq!(task.slot_id, slot_to_build);
        assert_eq!(task.name, BuildingName::Barracks);

        uow_assert1.rollback().await?;
        (job.clone(), village.id)
    };

    // --- 4. ACT (Phase 2): Simulate worker processing job ---
    let worker = Arc::new(JobWorker::new(
        uow_provider.clone(),
        app_registry,
        config.clone(),
    ));
    worker.process_jobs(&vec![job_to_run.clone()]).await?;

    // --- 5. ASSERT (Phase 2): Check final DB state ---
    {
        let uow_assert2 = uow_provider.begin().await?;

        // Check village
        let final_village = uow_assert2.villages().get_by_id(village_id).await?;
        let new_building = final_village.get_building_by_slot_id(slot_to_build);
        assert!(new_building.is_some(), "Building should now exist");
        assert_eq!(
            new_building.unwrap().building.level,
            1,
            "Building should be level 1"
        );

        // Check job
        let final_job = uow_assert2.jobs().get_by_id(job_to_run.id).await?;
        assert_eq!(
            final_job.status,
            JobStatus::Completed,
            "Job should be completed"
        );

        uow_assert2.rollback().await?;
    }

    Ok(())
}
