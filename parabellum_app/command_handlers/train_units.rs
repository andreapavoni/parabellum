use std::sync::Arc;

use parabellum_core::{ApplicationError, GameError};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::TrainUnits},
    jobs::{Job, JobPayload, tasks::TrainUnitsTask},
    uow::UnitOfWork,
};

pub struct TrainUnitsCommandHandler {}

impl Default for TrainUnitsCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

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
        config: &Arc<Config>,
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

        let (slot_id, unit_name, time_per_unit) = village.init_unit_training(
            command.unit_idx,
            &command.building_name,
            command.quantity,
            config.speed,
        )?;
        village_repo.save(&village).await?;

        let payload = TrainUnitsTask {
            slot_id,
            unit: unit_name,
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parabellum_core::Result;
    use parabellum_game::{
        models::{buildings::Building, village::Village},
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
    use crate::test_utils::tests::MockUnitOfWork;

    fn setup_village_with_barracks() -> Result<(Player, Village, Arc<Config>)> {
        let config = Arc::new(Config::from_env());
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
        village.set_academy_research_for_test(&UnitName::Legionnaire, true);

        let barracks =
            Building::new(BuildingName::Barracks, config.speed).at_level(10, config.speed)?;
        village.add_building_at_slot(barracks, 20)?;
        village.store_resources(&ResourceGroup(1000, 1000, 1000, 1000));

        Ok((player, village, config))
    }

    fn setup_village_with_stable() -> Result<(Player, Village, Arc<Config>)> {
        let config = Arc::new(Config::from_env());
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        let valley = valley_factory(Default::default());
        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            valley: Some(valley),
            ..Default::default()
        });

        let granary =
            Building::new(BuildingName::Granary, config.speed).at_level(20, config.speed)?;
        village.add_building_at_slot(granary, 20)?;

        let warehouse =
            Building::new(BuildingName::Warehouse, config.speed).at_level(20, config.speed)?;
        village.add_building_at_slot(warehouse, 21)?;

        let stable = Building::new(BuildingName::Stable, config.speed).at_level(1, config.speed)?;
        village.add_building_at_slot(stable, 22)?;
        village.set_academy_research_for_test(&UnitName::Pathfinder, true);
        village.store_resources(&ResourceGroup(10000, 10000, 10000, 10000));

        Ok((player, village, config))
    }

    #[tokio::test]
    async fn test_train_units_handler_success() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let job_repo = mock_uow.jobs();
        let village_repo = mock_uow.villages();

        let (player, village, config) = setup_village_with_barracks()?;
        let village_id = village.id;
        village_repo.save(&village).await.unwrap();

        let handler = TrainUnitsCommandHandler::new();

        let command = TrainUnits {
            player_id: player.id,
            village_id,
            unit_idx: 0,
            quantity: 5,
            building_name: BuildingName::Barracks,
        };

        handler.handle(command, &mock_uow, &config).await?;

        let saved_villages = village_repo.list_by_player_id(player.id).await?;
        assert_eq!(saved_villages.len(), 1, "Village should be saved once");
        let saved_village = &saved_villages[0];

        assert_eq!(
            saved_village.stored_resources().lumber(),
            800 - (120 * 5),
            "Lumber not deducted correctly"
        );
        assert_eq!(
            saved_village.stored_resources().clay(),
            800 - (100 * 5),
            "Clay not deducted correctly"
        );
        assert_eq!(
            saved_village.stored_resources().iron(),
            800 - (150 * 5),
            "Iron not deducted correctly"
        );
        assert_eq!(
            saved_village.stored_resources().crop(),
            800 - (30 * 5),
            "Crop not deducted correctly"
        );

        // Check if job was created
        let added_jobs = job_repo.list_by_player_id(player.id).await?;
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
        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_handler_not_enough_resources() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let job_repo = mock_uow.jobs();
        let village_repo = mock_uow.villages();

        let (player, mut village, config) = setup_village_with_barracks()?;
        village.store_resources(&ResourceGroup(10, 0, 0, 0));
        let village_id = village.id;
        village_repo.save(&village).await.unwrap();

        let handler = TrainUnitsCommandHandler::new();

        let command = TrainUnits {
            player_id: player.id,
            village_id,
            unit_idx: 0,
            quantity: 10,
            building_name: BuildingName::Barracks,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(result.err().unwrap().to_string(), "Not enough resources");
        assert_eq!(job_repo.list_by_player_id(player.id).await?.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_handler_missing_building() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());

        let village_repo = mock_uow.villages();
        let (player, mut village, config) = setup_village_with_barracks()?;

        village.remove_building_at_slot(20, config.speed)?;
        village_repo.save(&village).await?;

        let handler = TrainUnitsCommandHandler::new();
        let command = TrainUnits {
            player_id: player.id,
            village_id: village.id,
            unit_idx: 0,
            quantity: 1,
            building_name: BuildingName::Barracks,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(
            result.err().unwrap().to_string(),
            "Building requirements not met: requires Barracks at level 1"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_handler_missing_research() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());

        let village_repo = mock_uow.villages();

        let (player, mut village, config) = setup_village_with_barracks()?;
        village.set_academy_research_for_test(&UnitName::Praetorian, false);
        village_repo.save(&village).await?;

        let handler = TrainUnitsCommandHandler::new();
        let command = TrainUnits {
            player_id: player.id,
            village_id: village.id,
            unit_idx: 1,
            quantity: 1,
            building_name: BuildingName::Barracks,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(
            result.err().unwrap().to_string(),
            "Unit Praetorian not yet researched in Academy"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_handler_stable_success() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let job_repo = mock_uow.jobs();
        let village_repo = mock_uow.villages();

        let (player, village, config) = setup_village_with_stable()?;
        let village_id = village.id;
        village_repo.save(&village).await?;

        let handler = TrainUnitsCommandHandler::new();

        let command = TrainUnits {
            player_id: player.id,
            village_id,
            unit_idx: 2,
            quantity: 5,
            building_name: BuildingName::Stable,
        };

        // Pathfinder: 170, 150, 20, 40
        let unit_cost = ResourceGroup(170, 150, 20, 40);
        let total_cost = ResourceGroup(
            unit_cost.0 * 5,
            unit_cost.1 * 5,
            unit_cost.2 * 5,
            unit_cost.3 * 5,
        );
        let initial_lumber = village.stored_resources().lumber();

        handler.handle(command, &mock_uow, &config).await?;
        let saved_village = village_repo.get_by_id(village_id).await?;
        assert_eq!(
            saved_village.stored_resources().lumber(),
            initial_lumber - total_cost.0,
            "Lumber not deducted"
        );
        assert_eq!(
            saved_village.stored_resources().clay(),
            10000 - total_cost.1
        );

        let added_jobs = job_repo.list_by_player_id(player.id).await?;
        assert_eq!(added_jobs.len(), 1, "Expected a job for stable");

        let job = &added_jobs[0];
        assert_eq!(job.task.task_type, "TrainUnits");

        let task: TrainUnitsTask = serde_json::from_value(job.task.data.clone())?;
        assert_eq!(task.unit, UnitName::Pathfinder, "Expected unit trained");
        assert_eq!(task.quantity, 5);
        assert_eq!(
            task.slot_id, 22,
            "Task should be linked to the right slot_id"
        );
        Ok(())
    }
}
