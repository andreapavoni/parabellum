use std::sync::Arc;

use parabellum_core::{ApplicationError, GameError};
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::TrainUnits},
    jobs::{Job, JobPayload, tasks::TrainUnitsTask},
    uow::UnitOfWork,
};

pub struct TrainUnitsCommandHandler {}

impl TrainUnitsCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<TrainUnits> for TrainUnitsCommandHandler {
    async fn handle(
        &self,
        command: TrainUnits,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let village_repo = uow.villages();
        let job_repo = uow.jobs();
        let mut village = village_repo.get_by_id(command.village_id).await?;

        if village.player_id != command.player_id {
            return Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: command.village_id,
                player_id: command.player_id,
            }));
        }

        let tribe_units = village.tribe.get_units();
        let unit = tribe_units
            .get(command.unit_idx as usize)
            .ok_or_else(|| ApplicationError::Game(GameError::InvalidUnitIndex(command.unit_idx)))?;

        if !village.academy_research[command.unit_idx as usize] {
            return Err(ApplicationError::Game(GameError::UnitNotResearched(
                unit.name.clone(),
            )));
        }

        let cost_per_unit = &unit.cost;
        let total_cost = ResourceGroup::new(
            cost_per_unit.resources.0 * command.quantity as u32,
            cost_per_unit.resources.1 * command.quantity as u32,
            cost_per_unit.resources.2 * command.quantity as u32,
            cost_per_unit.resources.3 * command.quantity as u32,
        );

        if !village.stocks.check_resources(&total_cost) {
            return Err(ApplicationError::Game(GameError::NotEnoughResources));
        }
        village.stocks.remove_resources(&total_cost);
        village_repo.save(&village).await?;

        let time_per_unit = cost_per_unit.time;
        let building = village
            .get_building_by_name(BuildingName::Barracks)
            .ok_or_else(|| {
                ApplicationError::Game(GameError::BuildingRequirementsNotMet {
                    building: BuildingName::Barracks,
                    level: 1,
                })
            })?;

        let payload = TrainUnitsTask {
            slot_id: building.slot_id,
            unit: unit.clone().name,
            quantity: command.quantity,
            time_per_unit: time_per_unit as i32,
        };

        let job_payload = JobPayload::new("TrainUnits", serde_json::to_value(&payload)?);
        // Schedule the *first* unit to be completed.
        let new_job = Job::new(
            command.player_id,
            command.village_id as i32,
            time_per_unit as i64,
            job_payload,
        );

        job_repo.add(&new_job).await?;

        Ok(())
    }
}

// 4. Tests
#[cfg(test)]
mod tests {
    use parabellum_game::{
        models::{
            buildings::Building,
            village::{Village, VillageBuilding},
        },
        test_utils::{
            PlayerFactoryOptions, VillageFactoryOptions, player_factory, valley_factory,
            village_factory,
        },
    };
    use parabellum_types::{
        army::UnitName,
        buildings::BuildingName,
        common::{Player, ResourceGroup},
        tribe::Tribe,
    };

    use super::*;
    use crate::test_utils::tests::{MockUnitOfWork, assert_handler_success};

    use std::sync::Arc;

    fn setup_village_with_barracks() -> (Player, Village, Arc<Config>) {
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let valley = valley_factory(Default::default());
        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            valley: Some(valley),
            ..Default::default()
        });

        village.academy_research[0] = true; // Research Legionnaire

        // Add barracks
        let barracks = VillageBuilding {
            slot_id: 20,
            building: Building::new(BuildingName::Barracks),
        };
        village.buildings.push(barracks);

        // Add resources
        village
            .stocks
            .store_resources(ResourceGroup(1000, 1000, 1000, 1000));
        village.update_state();

        let config = Arc::new(Config::from_env());

        (player, village, config)
    }

    #[tokio::test]
    async fn test_train_units_handler_success() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let job_repo = mock_uow.jobs();
        let village_repo = mock_uow.villages();

        let (player, village, config) = setup_village_with_barracks();
        let village_id = village.id;
        village_repo.save(&village).await.unwrap();

        let handler = TrainUnitsCommandHandler::new();

        let command = TrainUnits {
            player_id: player.id,
            village_id: village_id,
            unit_idx: 0,
            quantity: 5,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert_handler_success(result);

        let saved_villages = village_repo.list_by_player_id(player.id).await.unwrap();
        assert_eq!(saved_villages.len(), 1, "Village should be saved once");
        let saved_village = &saved_villages[0];

        assert_eq!(
            saved_village.stocks.lumber,
            800 - (120 * 5),
            "Lumber not deducted correctly"
        );
        assert_eq!(
            saved_village.stocks.clay,
            800 - (100 * 5),
            "Clay not deducted correctly"
        );
        assert_eq!(
            saved_village.stocks.iron,
            800 - (150 * 5),
            "Iron not deducted correctly"
        );
        assert_eq!(
            saved_village.stocks.crop,
            800 - (30 * 5) as i64,
            "Crop not deducted correctly"
        );

        // Check if job was created
        let added_jobs = job_repo.list_by_player_id(player.id).await.unwrap();
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.player_id, player.id);
        assert_eq!(job.village_id, village_id as i32);

        assert_eq!(
            job.task.task_type, "TrainUnits",
            "Job task is not TrainUnitsTask"
        );

        let task: TrainUnitsTask = serde_json::from_value(job.task.data.clone())
            .expect("Failed to deserialize job task data");

        assert_eq!(task.unit, UnitName::Legionnaire);
        assert_eq!(task.quantity, 5);
    }

    #[tokio::test]
    async fn test_train_units_handler_not_enough_resources() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let job_repo = mock_uow.jobs();
        let village_repo = mock_uow.villages();

        let (player, mut village, config) = setup_village_with_barracks();
        village.stocks.lumber = 10; // Not enough lumber
        let village_id = village.id;
        village_repo.save(&village).await.unwrap();

        let handler = TrainUnitsCommandHandler::new();

        let command = TrainUnits {
            player_id: player.id,
            village_id: village_id,
            unit_idx: 0,
            quantity: 10,
        };

        let result = handler.handle(command, &mock_uow, &config).await;

        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(result.err().unwrap().to_string(), "Not enough resources");
        assert_eq!(
            job_repo.list_by_player_id(player.id).await.unwrap().len(),
            0
        );
    }

    #[tokio::test]
    async fn test_train_units_handler_missing_building() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());

        let village_repo = mock_uow.villages();

        let (player, mut village, config) = setup_village_with_barracks();

        village
            .buildings
            .retain(|vb| vb.building.name != BuildingName::Barracks);

        let village_id = village.id;
        village_repo.save(&village).await.unwrap();

        let handler = TrainUnitsCommandHandler::new();

        let command = TrainUnits {
            player_id: player.id,
            village_id: village_id,
            unit_idx: 0,
            quantity: 1,
        };

        let result = handler.handle(command, &mock_uow, &config).await;

        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(
            result.err().unwrap().to_string(),
            "Building requirements not met: requires Barracks at level 1"
        );
    }

    #[tokio::test]
    async fn test_train_units_handler_missing_research() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());

        let village_repo = mock_uow.villages();

        let (player, mut village, config) = setup_village_with_barracks();

        village
            .buildings
            .retain(|vb| vb.building.name != BuildingName::Barracks);

        let village_id = village.id;
        village_repo.save(&village).await.unwrap();

        let handler = TrainUnitsCommandHandler::new();

        let command = TrainUnits {
            player_id: player.id,
            village_id: village_id,
            unit_idx: 1,
            quantity: 1,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(
            result.err().unwrap().to_string(),
            "Unit Praetorian not yet researched in Academy"
        );
    }
}
