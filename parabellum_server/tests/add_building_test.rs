mod test_utils;

use parabellum_app::{
    command_handlers::AddBuildingCommandHandler,
    cqrs::commands::AddBuilding,
    jobs::{JobStatus, tasks::AddBuildingTask},
};
use parabellum_core::Result;
use parabellum_game::models::buildings::Building;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup, tribe::Tribe};

use crate::test_utils::tests::{setup_app, setup_player_party};

#[tokio::test]
async fn test_build() -> Result<()> {
    let (app, worker, uow_provider, config) = setup_app().await?;
    let (player, mut village, _, _) =
        setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

    {
        let uow = uow_provider.begin().await?;

        village.set_building_level_at_slot(19, 3, config.speed)?;
        let rally_point =
            Building::new(BuildingName::RallyPoint, config.speed).at_level(1, config.speed)?;
        village.add_building_at_slot(rally_point, 39).unwrap();
        village.store_resources(&ResourceGroup(1000, 1000, 1000, 1000));

        uow.villages().save(&village).await?;

        uow.commit().await?;
    }

    let cost = Building::new(BuildingName::Barracks, config.speed).cost();
    let initial_lumber = village.stored_resources().lumber();
    let slot_to_build: u8 = 22;

    let command = AddBuilding {
        player_id: player.id,
        village_id: village.id,
        slot_id: slot_to_build,
        name: BuildingName::Barracks,
    };
    let handler = AddBuildingCommandHandler::new();
    app.execute(command, handler).await?;

    let (job_to_run, village_id) = {
        let uow_assert1 = uow_provider.begin().await?;
        let updated_village = uow_assert1.villages().get_by_id(village.id).await?;
        assert_eq!(
            updated_village.stored_resources().lumber(),
            initial_lumber - cost.resources.0,
            "Resources should be deducted"
        );
        assert!(
            updated_village
                .get_building_by_slot_id(slot_to_build)
                .is_none(),
            "Building should NOT exist yet"
        );

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

    worker.process_jobs(&vec![job_to_run.clone()]).await?;
    {
        let uow_assert2 = uow_provider.begin().await?;
        let final_village = uow_assert2.villages().get_by_id(village_id).await?;
        let new_building = final_village.get_building_by_slot_id(slot_to_build);
        assert!(new_building.is_some(), "Building should now exist");
        assert_eq!(
            new_building.unwrap().building.level,
            1,
            "Building should be level 1"
        );

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
