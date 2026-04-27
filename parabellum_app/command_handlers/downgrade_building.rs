use std::sync::Arc;

use parabellum_types::buildings::BuildingName;
use parabellum_types::{
    Result,
    errors::{ApplicationError, GameError},
};

use crate::{
    command_handlers::helpers::{
        BuildingQueueJobPlan, build_scheduled_building_queue_job, building_queue_jobs,
    },
    config::Config,
    cqrs_es::building_queue::next_downgrade_target_level_via_cqrs,
    cqrs::{CommandHandler, commands::DowngradeBuilding},
    jobs::tasks::BuildingDowngradeTask,
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
        let mb_level = village.main_building_level();
        let active_jobs = job_repo
            .list_active_jobs_by_village(command.village_id as i32)
            .await?;
        let building_jobs = building_queue_jobs(active_jobs);

        let vb = village
            .get_building_by_slot_id(command.slot_id)
            .ok_or(GameError::EmptySlot {
                slot_id: command.slot_id,
            })?;

        if village
            .get_building_by_name(&BuildingName::MainBuilding)
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
        let target_level = next_downgrade_target_level_via_cqrs(
            command.village_id,
            command.slot_id,
            vb.building.name.clone(),
            current_level,
        )
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let target_level_building = vb.building.at_level(target_level, config.speed)?;
        let build_time_secs =
            target_level_building.calculate_build_time_secs(&config.speed, &mb_level) as i64;

        let payload = BuildingDowngradeTask {
            slot_id: command.slot_id,
            building_name: vb.building.name.clone(),
            level: target_level,
        };

        let new_job = build_scheduled_building_queue_job(
            command.player_id,
            command.village_id as i32,
            &building_jobs,
            build_time_secs,
            BuildingQueueJobPlan::Downgrade(payload),
        )?;
        job_repo.add(&new_job).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use parabellum_game::models::village::Village;
    use parabellum_game::test_utils::{
        PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
    };
    use parabellum_types::Result;
    use parabellum_types::{common::Player, tribe::Tribe};

    use super::*;
    use crate::{
        config::Config,
        cqrs::commands::DowngradeBuilding,
        jobs::{Job, JobPayload, tasks::{BuildingDowngradeTask, BuildingUpgradeTask}},
        test_utils::tests::MockUnitOfWork,
        uow::UnitOfWork,
    };
    use std::sync::Arc;

    fn setup_village_for_downgrade() -> Result<(Player, Village, Arc<Config>, u8)> {
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
        village.set_building_level_at_slot(slot_id, 10, config.speed)?;
        Ok((player, village, config, slot_id))
    }

    #[tokio::test]
    async fn test_downgrade_building_handler_success_creates_job() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config, slot_id) = setup_village_for_downgrade()?;

        let village_id = village.id;
        let player_id = player.id;
        let initial_population = village.population;

        let initial_building = village.get_building_by_slot_id(slot_id).unwrap();

        mock_uow.villages().save(&village).await?;

        let handler = DowngradeBuildingCommandHandler::new();
        let command = DowngradeBuilding {
            player_id,
            village_id,
            slot_id,
        };

        handler.handle(command.clone(), &mock_uow, &config).await?;

        let saved_village = mock_uow.villages().get_by_id(village_id).await?;
        let building_in_db = saved_village.get_building_by_slot_id(slot_id).unwrap();
        assert_eq!(
            building_in_db.building.level, initial_building.building.level,
            "Building shouldn't be downgraded"
        );
        assert_eq!(saved_village.population, initial_population);

        let added_jobs = mock_uow.jobs().list_by_player_id(player_id).await?;
        assert_eq!(added_jobs.len(), 1, "Expected 1 job created");

        let job = &added_jobs[0];
        assert_eq!(job.task.task_type, "BuildingDowngrade");

        let task: BuildingDowngradeTask = serde_json::from_value(job.task.data.clone())?;
        assert_eq!(task.slot_id, slot_id);
        assert_eq!(
            task.level,
            building_in_db.building.level - 1,
            "Task should contain target level at N-1"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_downgrade_building_handler_queues_after_existing_slot_job() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config, slot_id) = setup_village_for_downgrade()?;
        let village_id = village.id;
        let player_id = player.id;
        mock_uow.villages().save(&village).await?;

        let queued_payload = JobPayload::new(
            "BuildingUpgrade",
            serde_json::to_value(BuildingUpgradeTask {
                slot_id,
                building_name: parabellum_types::buildings::BuildingName::MainBuilding,
                level: 11,
            })?,
        );
        let queued_deadline = Utc::now() + Duration::minutes(5);
        let queued_job = Job::with_deadline(player_id, village_id as i32, queued_payload, queued_deadline);
        mock_uow.jobs().add(&queued_job).await?;

        let handler = DowngradeBuildingCommandHandler::new();
        let command = DowngradeBuilding {
            player_id,
            village_id,
            slot_id,
        };
        handler.handle(command, &mock_uow, &config).await?;

        let mut jobs = mock_uow.jobs().list_by_player_id(player_id).await?;
        jobs.sort_by_key(|job| job.completed_at);
        assert_eq!(jobs.len(), 2);
        let scheduled = jobs
            .iter()
            .find(|job| job.id != queued_job.id)
            .expect("new downgrade job");
        assert_eq!(scheduled.task.task_type, "BuildingDowngrade");
        assert!(
            scheduled.completed_at > queued_job.completed_at,
            "downgrade should be scheduled after existing same-slot queue job"
        );
        Ok(())
    }
}
