use std::sync::Arc;

use parabellum_core::{ApplicationError, GameError, Result};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::UpgradeBuilding},
    jobs::{Job, JobPayload, tasks::BuildingUpgradeTask},
    uow::UnitOfWork,
};

pub struct UpgradeBuildingCommandHandler {}

impl Default for UpgradeBuildingCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl UpgradeBuildingCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<UpgradeBuilding> for UpgradeBuildingCommandHandler {
    async fn handle(
        &self,
        command: UpgradeBuilding,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let village_repo = uow.villages();
        let job_repo = uow.jobs();
        let mut village = village_repo.get_by_id(command.village_id).await?;
        let mb_level = village.main_building_level();

        let vb = village
            .get_building_by_slot_id(command.slot_id)
            .ok_or_else(|| GameError::EmptySlot {
                slot_id: command.slot_id,
            })?;

        let next_level = vb.building.level + 1;
        let next_level_building = vb.building.at_level(next_level, config.speed)?;
        let cost = next_level_building.cost();
        let build_time_secs =
            next_level_building.calculate_build_time_secs(&config.speed, &mb_level) as i64;

        village.deduct_resources(&cost.resources)?;
        village_repo.save(&village).await?;

        let payload = BuildingUpgradeTask {
            slot_id: command.slot_id,
            building_name: next_level_building.name.clone(),
            level: next_level,
        };

        let job_payload = JobPayload::new("BuildingUpgrade", serde_json::to_value(&payload)?);
        let new_job = Job::new(
            command.player_id,
            command.village_id as i32,
            build_time_secs,
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
        models::village::Village,
        test_utils::{
            PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
        },
    };
    use parabellum_types::{
        buildings::BuildingName,
        common::{Player, ResourceGroup},
        tribe::Tribe,
    };

    use super::*;
    use crate::{
        config::Config, cqrs::commands::UpgradeBuilding, jobs::tasks::BuildingUpgradeTask,
        test_utils::tests::MockUnitOfWork, uow::UnitOfWork,
    };

    fn setup_village_for_upgrade() -> Result<(Player, Village, Arc<Config>, u8)> {
        let config = Arc::new(Config::from_env());
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });

        let main_building_slot = 19;
        village
            .set_building_level_at_slot(main_building_slot, 1, config.speed)
            .unwrap();
        village.store_resources(&ResourceGroup(1000, 1000, 1000, 1000));

        Ok((player, village, config, main_building_slot))
    }

    #[tokio::test]
    async fn test_upgrade_building_handler_success() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config, slot_id) = setup_village_for_upgrade()?;

        let village_id = village.id;
        let player_id = player.id;
        let initial_lumber = village.stored_resources().lumber();
        let initial_population = village.population;

        mock_uow.villages().save(&village).await?;

        let handler = UpgradeBuildingCommandHandler::new();
        let command = UpgradeBuilding {
            player_id,
            village_id,
            slot_id,
        };

        let l2_building = village
            .get_building_by_slot_id(slot_id)
            .unwrap()
            .building
            .at_level(2, config.speed)
            .unwrap();
        let cost = l2_building.cost();

        handler.handle(command.clone(), &mock_uow, &config).await?;
        let saved_village = mock_uow.villages().get_by_id(village_id).await?;
        assert_eq!(
            saved_village.stored_resources().lumber(),
            initial_lumber - cost.resources.0,
            "No resources withdrawn from stocks"
        );

        let building_in_db = saved_village.get_building_by_slot_id(slot_id).unwrap();
        assert_eq!(
            building_in_db.building.level, 1,
            "Expected building level at {}, got {}",
            1, building_in_db.building.level
        );
        assert_eq!(
            saved_village.population, initial_population,
            "Expected population at {}, got {}",
            initial_population, saved_village.population
        );

        let added_jobs = mock_uow.jobs().list_by_player_id(player_id).await?;
        assert_eq!(added_jobs.len(), 1, "Should have created 1 job");
        let job = &added_jobs[0];
        assert_eq!(job.task.task_type, "BuildingUpgrade");

        let task: BuildingUpgradeTask = serde_json::from_value(job.task.data.clone())?;
        assert_eq!(task.slot_id, slot_id);
        assert_eq!(task.building_name, BuildingName::MainBuilding);
        assert_eq!(
            task.level, 2,
            "Expected task having level {}, got {}",
            2, task.level
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_upgrade_building_handler_not_enough_resources() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config, slot_id) = setup_village_for_upgrade()?;

        assert!(
            village
                .deduct_resources(&ResourceGroup(800, 800, 800, 800))
                .is_ok(),
            "Village should have enough resources"
        );
        mock_uow.villages().save(&village).await?;

        let handler = UpgradeBuildingCommandHandler::new();
        let command = UpgradeBuilding {
            player_id: player.id,
            village_id: village.id,
            slot_id,
        };

        let result = handler.handle(command.clone(), &mock_uow, &config).await;

        assert!(result.is_err(), "Expected handler to fail");
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::NotEnoughResources.to_string()
        );

        let added_jobs = mock_uow.jobs().list_by_player_id(player.id).await?;
        assert_eq!(added_jobs.len(), 0, "No jobs should be created");
        Ok(())
    }
}
