use std::sync::Arc;

use crate::{
    Result,
    config::Config,
    cqrs::{CommandHandler, commands::AddBuilding},
    game::{GameError, models::buildings::Building},
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
        command: AddBuilding,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<()> {
        let villages_repo = uow.villages();

        let mut village = villages_repo.get_by_id(command.village_id).await?;

        if village.buildings.len() == 40 {
            return Err(GameError::VillageSlotsFull.into());
        }
        if village.get_building_by_slot_id(command.slot_id).is_some() {
            return Err(GameError::SlotOccupied {
                slot_id: command.slot_id,
            }
            .into());
        }

        let building = Building::new(command.name.clone());

        Building::validate_build(
            &building,
            &village.tribe,
            &village.buildings,
            village.is_capital,
        )?;

        let cost = building.cost();
        if !village.stocks.check_resources(&cost.resources) {
            return Err(GameError::NotEnoughResources.into());
        }

        village.stocks.remove_resources(&cost.resources);
        village.update_state();
        villages_repo.save(&village).await?;

        let payload = AddBuildingTask {
            village_id: village.id as i32,
            slot_id: command.slot_id,
            name: building.clone().name,
        };

        let job_payload = JobPayload::new("AddBuilding", serde_json::to_value(&payload)?);
        let new_job = Job::new(
            command.player_id,
            command.village_id as i32,
            building.calculate_build_time_secs(config.speed.clone() as u8) as i64,
            job_payload,
        );
        uow.jobs().add(&new_job).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::test_utils::tests::{MockUnitOfWork, assert_handler_success},
        config::Config,
        game::{
            models::{
                Player, Tribe,
                buildings::{Building, BuildingName},
                common::ResourceGroup,
                village::Village,
            },
            test_utils::{
                PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
            },
        },
        jobs::tasks::AddBuildingTask,
    };
    use std::sync::Arc;

    fn setup_village() -> (Player, Village, Arc<Config>) {
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });

        // Add Main Building Lvl 3 (requirement for Barracks)
        village.set_building_level_at_slot(19, 3).unwrap();

        // Add Rally Point Lvl 1 (requirement for Barracks)
        let rally_point = Building::new(BuildingName::RallyPoint).at_level(1).unwrap();
        village.add_building_at_slot(rally_point, 39).unwrap(); // Slot 39 for Rally Point

        village
            .stocks
            .store_resources(ResourceGroup(1000, 1000, 1000, 1000));
        village.update_state();

        let config = Arc::new(Config::from_env());
        (player, village, config)
    }

    #[tokio::test]
    async fn test_add_building_handler_success() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config) = setup_village();
        let village_id = village.id;
        let player_id = player.id;

        mock_uow.villages().save(&village).await.unwrap();

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id,
            village_id,
            slot_id: 22,
            name: BuildingName::Barracks,
        };

        let result = handler.handle(command, &mock_uow, &config).await;

        assert_handler_success(result);

        // Check if resources were deducted
        let saved_village = mock_uow.villages().get_by_id(village_id).await.unwrap();
        let cost = Building::new(BuildingName::Barracks).cost();

        assert_eq!(
            saved_village.stocks.lumber,
            800 - cost.resources.0,
            "Lumber not deducted correctly"
        );

        // Check if job was created
        let added_jobs = mock_uow.jobs().list_by_player_id(player_id).await.unwrap();
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.task.task_type, "AddBuilding");
        let task: AddBuildingTask = serde_json::from_value(job.task.data.clone()).unwrap();
        assert_eq!(task.slot_id, 22);
        assert_eq!(task.name, BuildingName::Barracks);

        // Check that building is NOT added to village yet
        assert!(
            saved_village.get_building_by_slot_id(22).is_none(),
            "Building should not be added by the command handler"
        );
    }

    #[tokio::test]
    async fn test_add_building_handler_not_enough_resources() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village();
        village.stocks = Default::default(); // No resources
        let village_id = village.id;
        let player_id = player.id;

        mock_uow.villages().save(&village).await.unwrap();

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
            GameError::NotEnoughResources.to_string()
        );

        // Check that no job was created
        let added_jobs = uow_box.jobs().list_by_player_id(player_id).await.unwrap();
        assert_eq!(added_jobs.len(), 0, "No job should be created");
    }

    #[tokio::test]
    async fn test_add_building_handler_slot_occupied() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village();
        let village_id = village.id;
        let player_id = player.id;

        // Manually occupy slot 22
        let cranny = Building::new(BuildingName::Cranny);
        village.add_building_at_slot(cranny, 22).unwrap();
        mock_uow.villages().save(&village).await.unwrap();

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
    }

    #[tokio::test]
    async fn test_add_building_handler_requirements_not_met() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village();

        // Remove Rally Point (requirement for Barracks)
        village
            .buildings
            .retain(|vb| vb.building.name != BuildingName::RallyPoint);

        let village_id = village.id;
        let player_id = player.id;

        mock_uow.villages().save(&village).await.unwrap();

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
    }
}
