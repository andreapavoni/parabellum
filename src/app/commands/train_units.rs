use std::sync::Arc;
use uuid::Uuid;

use crate::{
    Result,
    error::ApplicationError,
    game::{
        GameError,
        models::{ResourceGroup, buildings::BuildingName},
    },
    jobs::{Job, JobPayload, tasks::TrainUnitsTask},
    repository::{JobRepository, VillageRepository},
};

#[derive(Debug, Clone)]
pub struct TrainUnitsCommand {
    pub player_id: Uuid,
    pub village_id: u32,
    pub unit_idx: u8,
    pub quantity: i32,
}

pub struct TrainUnitsCommandHandler<'a> {
    village_repo: Arc<dyn VillageRepository + 'a>,
    job_repo: Arc<dyn JobRepository + 'a>,
}

impl<'a> TrainUnitsCommandHandler<'a> {
    pub fn new(
        village_repo: Arc<dyn VillageRepository + 'a>,
        job_repo: Arc<dyn JobRepository + 'a>,
    ) -> Self {
        Self {
            village_repo,
            job_repo,
        }
    }

    pub async fn handle(&self, command: TrainUnitsCommand) -> Result<(), ApplicationError> {
        let mut village = self.village_repo.get_by_id(command.village_id).await?;

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

        // if !village.stocks.check_resources(&total_cost) {
        //     return Err(ApplicationError::Game(GameError::NotEnoughResources));
        // }

        village.stocks.withdraw_resources(&total_cost)?;
        self.village_repo.save(&village).await?;

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

        self.job_repo.add(&new_job).await?;

        Ok(())
    }
}

// 4. Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::test_utils::tests::{MockJobRepository, MockVillageRepository},
        game::{
            models::{
                Player, Tribe,
                army::UnitName,
                buildings::Building,
                village::{Village, VillageBuilding},
            },
            test_factories::{
                PlayerFactoryOptions, VillageFactoryOptions, player_factory, valley_factory,
                village_factory,
            },
        },
    };

    use std::sync::Arc;

    fn setup_village_with_barracks() -> (Player, Village) {
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
            slot_id: 20, // Example slot
            building: Building::new(BuildingName::Barracks),
        };
        village.buildings.push(barracks);

        // Add resources
        village
            .stocks
            .store_resources(ResourceGroup(1000, 1000, 1000, 1000));
        village.update_state();

        (player, village)
    }

    #[tokio::test]
    async fn test_train_units_handler_success() {
        // 1. Setup
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());

        let (player, village) = setup_village_with_barracks();
        let village_id = village.id;
        mock_village_repo.add_village(village);

        let handler =
            TrainUnitsCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = TrainUnitsCommand {
            player_id: player.id,
            village_id: village_id,
            unit_idx: 0,
            quantity: 5,
        };

        // 2. Act
        let result = handler.handle(command).await;

        // 3. Assert
        assert!(result.is_ok(), "Handler should execute successfully");

        // Check if resources were deducted
        let saved_villages = mock_village_repo.saved_villages();
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
        let added_jobs = mock_job_repo.get_added_jobs();
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
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());

        let (player, mut village) = setup_village_with_barracks();
        village.stocks.lumber = 10; // Not enough lumber
        let village_id = village.id;
        mock_village_repo.add_village(village);

        let handler =
            TrainUnitsCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = TrainUnitsCommand {
            player_id: player.id,
            village_id: village_id,
            unit_idx: 0,
            quantity: 10,
        };

        let result = handler.handle(command).await;

        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(result.err().unwrap().to_string(), "Not enough resources");
        assert_eq!(mock_job_repo.get_added_jobs().len(), 0);
    }

    #[tokio::test]
    async fn test_train_units_handler_missing_building() {
        // 1. Setup
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());

        let (player, mut village) = setup_village_with_barracks();

        village
            .buildings
            .retain(|vb| vb.building.name != BuildingName::Barracks);

        let village_id = village.id;
        mock_village_repo.add_village(village);

        let handler =
            TrainUnitsCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = TrainUnitsCommand {
            player_id: player.id,
            village_id: village_id,
            unit_idx: 0,
            quantity: 1,
        };

        let result = handler.handle(command).await;

        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(
            result.err().unwrap().to_string(),
            "Building requirements not met: requires Barracks at level 1"
        );
    }

    #[tokio::test]
    async fn test_train_units_handler_missing_research() {
        // 1. Setup
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());

        let (player, mut village) = setup_village_with_barracks();

        village
            .buildings
            .retain(|vb| vb.building.name != BuildingName::Barracks);

        let village_id = village.id;
        mock_village_repo.add_village(village);

        let handler =
            TrainUnitsCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = TrainUnitsCommand {
            player_id: player.id,
            village_id: village_id,
            unit_idx: 1,
            quantity: 1,
        };

        // 2. Act
        let result = handler.handle(command).await;

        // 3. Assert
        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(
            result.err().unwrap().to_string(),
            "Unit Praetorian not yet researched in Academy"
        );
    }
}
