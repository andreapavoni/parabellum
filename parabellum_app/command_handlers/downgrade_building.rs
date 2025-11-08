use std::sync::Arc;

use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_types::buildings::BuildingName;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::DowngradeBuilding},
    jobs::{Job, JobPayload, tasks::BuildingDowngradeTask},
    uow::UnitOfWork,
};

pub struct DowngradeBuildingCommandHandler {}

impl Default for DowngradeBuildingCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DowngradeBuildingCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<DowngradeBuilding> for DowngradeBuildingCommandHandler {
    async fn handle(
        &self,
        command: DowngradeBuilding,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<()> {
        let village_repo = uow.villages();
        let job_repo = uow.jobs();
        let village = village_repo.get_by_id(command.village_id).await?;
        let mb_level = village.get_main_building_level();

        let vb = village
            .get_building_by_slot_id(command.slot_id)
            .ok_or_else(|| GameError::EmptySlot {
                slot_id: command.slot_id,
            })?;

        if village
            .get_building_by_name(BuildingName::MainBuilding)
            .map_or(0, |vb| vb.building.level)
            < 10
        {
            return Err(GameError::BuildingRequirementsNotMet {
                building: BuildingName::MainBuilding,
                level: 10,
            }
            .into());
        }

        let current_level = vb.building.level;
        if current_level == 0 {
            return Err(ApplicationError::Game(GameError::InvalidBuildingLevel(
                0,
                vb.building.name,
            )));
        }
        let target_level = current_level - 1;
        let target_level_building = vb.building.at_level(target_level, config.speed)?;
        let build_time_secs =
            target_level_building.calculate_build_time_secs(&config.speed, &mb_level) as i64;

        let payload = BuildingDowngradeTask {
            slot_id: command.slot_id,
            building_name: vb.building.name.clone(),
            level: target_level,
        };

        let job_payload = JobPayload::new("BuildingDowngrade", serde_json::to_value(&payload)?);
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
    use parabellum_game::models::village::Village;
    use parabellum_game::test_utils::{
        PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
    };
    use parabellum_types::{common::Player, tribe::Tribe};

    use super::*;
    use crate::{
        config::Config,
        cqrs::commands::DowngradeBuilding,
        jobs::tasks::BuildingDowngradeTask,
        test_utils::tests::{MockUnitOfWork, assert_handler_success},
        uow::UnitOfWork,
    };
    use std::sync::Arc;

    fn setup_village_for_downgrade() -> (Player, Village, Arc<Config>, u8) {
        let config = Arc::new(Config::from_env());
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });

        let slot_id = 19;
        village
            .set_building_level_at_slot(slot_id, 10, config.speed)
            .unwrap();

        (player, village, config, slot_id)
    }

    #[tokio::test]
    async fn test_downgrade_building_handler_success_creates_job() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config, slot_id) = setup_village_for_downgrade();

        let village_id = village.id;
        let player_id = player.id;
        let initial_population = village.population;

        let initial_building = village.get_building_by_slot_id(slot_id).unwrap();

        mock_uow.villages().save(&village).await.unwrap();

        let handler = DowngradeBuildingCommandHandler::new();
        let command = DowngradeBuilding {
            player_id,
            village_id,
            slot_id,
        };

        let result = handler.handle(command.clone(), &mock_uow, &config).await;
        assert_handler_success(result);

        let saved_village = mock_uow.villages().get_by_id(village_id).await.unwrap();
        let building_in_db = saved_village.get_building_by_slot_id(slot_id).unwrap();
        assert_eq!(
            building_in_db.building.level, initial_building.building.level,
            "Building shouldn't be downgraded"
        );
        assert_eq!(saved_village.population, initial_population);

        let added_jobs = mock_uow.jobs().list_by_player_id(player_id).await.unwrap();
        assert_eq!(added_jobs.len(), 1, "Expected 1 job created");

        let job = &added_jobs[0];
        assert_eq!(job.task.task_type, "BuildingDowngrade");

        let task: BuildingDowngradeTask = serde_json::from_value(job.task.data.clone()).unwrap();
        assert_eq!(task.slot_id, slot_id);
        assert_eq!(
            task.level,
            building_in_db.building.level - 1,
            "Task should contain target level at N-1"
        );
    }
}
