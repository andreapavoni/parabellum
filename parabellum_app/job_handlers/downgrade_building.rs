use async_trait::async_trait;
use parabellum_core::{ApplicationError, GameError};
use parabellum_types::buildings::BuildingGroup;
use tracing::{info, instrument};

use crate::jobs::{
    Job,
    handler::{JobHandler, JobHandlerContext},
    tasks::BuildingDowngradeTask,
};

pub struct DowngradeBuildingJobHandler {
    payload: BuildingDowngradeTask,
}

impl DowngradeBuildingJobHandler {
    pub fn new(payload: BuildingDowngradeTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for DowngradeBuildingJobHandler {
    #[instrument(skip_all, fields(
        task_type = "DowngradeBuilding",
        slot_id = self.payload.slot_id,
        target_level = self.payload.level,
        village_id = job.village_id,
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Executing DowngradeBuilding job");

        let village_id = job.village_id as u32;
        let village_repo = ctx.uow.villages();
        let mut village = village_repo.get_by_id(village_id).await?;

        let vb = village
            .get_building_by_slot_id(self.payload.slot_id)
            .ok_or_else(|| GameError::EmptySlot {
                slot_id: self.payload.slot_id,
            })?;

        if self.payload.level == 0 && vb.building.group != BuildingGroup::Resources {
            info!(
                "Demolishing {:?} in village {} completely",
                vb.building.name, village_id
            );
            village.remove_building_at_slot(self.payload.slot_id, ctx.config.speed)?;
        } else {
            info!(
                "Downgrading {:?} in village {} to level {}",
                vb.building.name, village_id, self.payload.level
            );
            village.set_building_level_at_slot(
                self.payload.slot_id,
                self.payload.level,
                ctx.config.speed,
            )?;
        }

        village_repo.save(&village).await?;
        Ok(())
    }
}

// Tests
#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::sync::Arc;

    use parabellum_core::Result;
    use parabellum_game::test_utils::{
        PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
    };
    use parabellum_types::{
        buildings::{BuildingGroup, BuildingName},
        tribe::Tribe,
    };

    use super::*;
    use crate::{
        config::Config,
        jobs::{Job, JobPayload},
        test_utils::tests::MockUnitOfWork,
        uow::UnitOfWork,
    };

    async fn setup_job_test(
        start_level: u8,
        slot_id: u8,
        name: BuildingName,
    ) -> Result<(
        Job,
        Arc<Config>,
        Box<dyn UnitOfWork<'static> + 'static>,
        u32,
        u32,
    )> {
        let config = Arc::new(Config::from_env());
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });

        village.set_building_level_at_slot(slot_id, start_level, config.speed)?;
        let initial_population = village.population;

        let village_id = village.id;
        let player_id = player.id;

        let mock_uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());
        mock_uow.villages().save(&village).await?;

        let payload = BuildingDowngradeTask {
            slot_id,
            building_name: name,
            level: start_level - 1, // Target level
        };
        let job_payload = JobPayload::new("DowngradeBuilding", json!(payload.clone()));
        let job = Job::new(player_id, village_id as i32, 0, job_payload);

        Ok((job, config, mock_uow, village_id, initial_population))
    }

    #[tokio::test]
    async fn test_downgrade_job_handler_success() -> Result<()> {
        // Test: MainBuilding L2 -> L1 (Slot 19)
        let (job, config, uow, village_id, initial_pop) =
            setup_job_test(2, 19, BuildingName::MainBuilding).await?;

        let handler =
            DowngradeBuildingJobHandler::new(serde_json::from_value(job.task.data.clone())?);
        let context = JobHandlerContext { uow, config };

        handler.handle(&context, &job).await?;
        let saved_village = context.uow.villages().get_by_id(village_id).await?;
        let building_in_db = saved_village.get_building_by_slot_id(19).unwrap();

        assert_eq!(
            building_in_db.building.level, 1,
            "Expected building at level {}, got {}",
            1, building_in_db.building.level
        );
        assert_eq!(
            saved_village.population,
            initial_pop - 1,
            "Expected village population at {}, got {}",
            initial_pop - 1,
            saved_village.population
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_downgrade_job_handler_resource_to_zero() -> Result<()> {
        let (job, config, uow, village_id, _initial_pop) =
            setup_job_test(1, 1, BuildingName::Woodcutter).await?;

        let handler =
            DowngradeBuildingJobHandler::new(serde_json::from_value(job.task.data.clone())?);
        let context = JobHandlerContext { uow, config };

        handler.handle(&context, &job).await?;
        let saved_village = context.uow.villages().get_by_id(village_id).await?;
        let building_in_db = saved_village.get_building_by_slot_id(1).unwrap();

        assert_eq!(
            building_in_db.building.level, 0,
            "Expected field resource at level 0, got {}",
            building_in_db.building.level
        );
        assert_eq!(building_in_db.building.group, BuildingGroup::Resources);
        Ok(())
    }

    #[tokio::test]
    async fn test_downgrade_job_handler_infra_to_zero_removes_it() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let player = player_factory(PlayerFactoryOptions::default());
        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });

        let slot_id = 20;
        let building = parabellum_game::models::buildings::Building::new(
            BuildingName::Warehouse,
            config.speed,
        );
        village.add_building_at_slot(building, slot_id)?;
        let initial_pop = village.population;
        let village_id = village.id;

        let mock_uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());
        mock_uow.villages().save(&village).await?;

        let payload = BuildingDowngradeTask {
            slot_id,
            building_name: BuildingName::Warehouse,
            level: 0,
        };
        let job_payload = JobPayload::new("DowngradeBuilding", json!(payload.clone()));
        let job = Job::new(player.id, village_id as i32, 0, job_payload);

        let handler = DowngradeBuildingJobHandler::new(payload);
        let context = JobHandlerContext {
            uow: mock_uow,
            config,
        };
        handler.handle(&context, &job).await?;

        let saved_village = context.uow.villages().get_by_id(village_id).await?;
        let building_in_db = saved_village.get_building_by_slot_id(slot_id);
        assert!(building_in_db.is_none(), "Building should be demolished");
        assert_eq!(
            saved_village.population,
            initial_pop - 1,
            "Expected village population at {}, got {}",
            initial_pop - 1,
            saved_village.population
        );
        Ok(())
    }
}
