use std::sync::Arc;

use parabellum_core::Result;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::AddBuilding},
    jobs::{Job, JobPayload, tasks::AddBuildingTask},
    uow::UnitOfWork,
};

pub struct AddBuildingCommandHandler {}

impl Default for AddBuildingCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl AddBuildingCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<AddBuilding> for AddBuildingCommandHandler {
    async fn handle(
        &self,
        cmd: AddBuilding,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<()> {
        let villages_repo = uow.villages();

        let mut village = villages_repo.get_by_id(cmd.village_id).await?;
        let build_time_secs =
            village.init_building_construction(cmd.slot_id, cmd.name.clone(), config.speed)?;
        villages_repo.save(&village).await?;

        let payload = AddBuildingTask {
            village_id: village.id as i32,
            slot_id: cmd.slot_id,
            name: cmd.name,
        };
        let job_payload = JobPayload::new("AddBuilding", serde_json::to_value(&payload)?);
        let new_job = Job::new(
            cmd.player_id,
            cmd.village_id as i32,
            build_time_secs as i64,
            job_payload,
        );
        uow.jobs().add(&new_job).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use parabellum_core::{GameError, Result};
    use parabellum_game::{
        models::{buildings::Building, village::Village},
        test_utils::setup_player_party,
    };
    use parabellum_types::{
        buildings::BuildingName,
        common::{Player, ResourceGroup},
        tribe::Tribe,
    };

    use super::*;
    use crate::{jobs::tasks::AddBuildingTask, test_utils::tests::MockUnitOfWork};
    use std::sync::Arc;

    fn setup_village(config: &Config) -> Result<(Player, Village, Arc<Config>)> {
        let (player, mut village, _, _) =
            setup_player_party(None, Tribe::Roman, [0; 10], false).unwrap();

        // main building is level 1 in slot 19 by default
        village.set_building_level_at_slot(19, 3, config.speed)?;

        let rally_point =
            Building::new(BuildingName::RallyPoint, config.speed).at_level(1, config.speed)?;
        village.add_building_at_slot(rally_point, 39)?;

        let config = Arc::new(Config::from_env());
        Ok((player, village, config))
    }

    #[tokio::test]
    async fn test_add_building_handler_success() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let config = Config::from_env();
        let (player, mut village, config) = setup_village(&config)?;
        let village_id = village.id;
        let player_id = player.id;

        village.store_resources(&ResourceGroup(1000, 1000, 1000, 1000));

        mock_uow.villages().save(&village).await?;

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id,
            village_id,
            slot_id: 22,
            name: BuildingName::Barracks,
        };

        handler.handle(command, &mock_uow, &config).await?;

        // Check if resources were deducted
        let saved_village = mock_uow.villages().get_by_id(village_id).await?;
        let cost = Building::new(BuildingName::Barracks, config.speed).cost();

        assert_eq!(
            saved_village.stored_resources().lumber(),
            800 - cost.resources.0,
            "Lumber not deducted correctly"
        );

        // Check if job was created
        let added_jobs = mock_uow.jobs().list_by_player_id(player_id).await?;
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.task.task_type, "AddBuilding");
        let task: AddBuildingTask = serde_json::from_value(job.task.data.clone())?;
        assert_eq!(task.slot_id, 22);
        assert_eq!(task.name, BuildingName::Barracks);

        // Check that building is NOT added to village yet
        assert!(
            saved_village.get_building_by_slot_id(22).is_none(),
            "Building should not be added by the command handler"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_add_building_handler_not_enough_resources() -> Result<()> {
        let config = Config::from_env();
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config) = setup_village(&config)?;

        mock_uow.villages().save(&village).await?;

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id: player.id,
            village_id: village.id,
            slot_id: 22,
            name: BuildingName::Barracks,
        };

        let uow_box = Box::new(mock_uow);
        let result = handler.handle(command, &uow_box, &config).await;

        assert!(result.is_err(), "Handler should fail");
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::NotEnoughResources.to_string()
        );

        // Check that no job was created
        let added_jobs = uow_box.jobs().list_by_player_id(player.id).await?;
        assert_eq!(added_jobs.len(), 0, "No job should be created");
        Ok(())
    }

    #[tokio::test]
    async fn test_add_building_handler_slot_occupied() -> Result<()> {
        let config = Config::from_env();
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village(&config)?;
        let village_id = village.id;
        let player_id = player.id;

        // Manually occupy slot 22
        let cranny = Building::new(BuildingName::Cranny, config.speed);
        village.add_building_at_slot(cranny, 22)?;
        mock_uow.villages().save(&village).await?;

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id,
            village_id,
            slot_id: 22, // Try to build on same slot
            name: BuildingName::Barracks,
        };

        let uow_box = Box::new(mock_uow);
        let result = handler.handle(command, &uow_box, &config).await;

        assert!(result.is_err(), "Handler should fail");
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::SlotOccupied { slot_id: 22 }.to_string()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_add_building_handler_requirements_not_met() -> Result<()> {
        let config = Config::from_env();
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village(&config)?;

        // Remove Rally Point at slot 39 (requirement for Barracks)
        village.remove_building_at_slot(39, config.speed)?;

        let village_id = village.id;
        let player_id = player.id;

        mock_uow.villages().save(&village).await?;

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id,
            village_id,
            slot_id: 22,
            name: BuildingName::Barracks,
        };

        let uow_box = Box::new(mock_uow);
        let result = handler.handle(command, &uow_box, &config).await;

        assert!(result.is_err(), "Handler should fail");
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::BuildingRequirementsNotMet {
                building: BuildingName::RallyPoint,
                level: 1,
            }
            .to_string()
        );
        Ok(())
    }
}
