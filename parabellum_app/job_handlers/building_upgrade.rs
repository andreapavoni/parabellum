use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_types::errors::ApplicationError;

use crate::{
    job_handlers::helpers::update_player_culture_points,
    jobs::{
        Job,
        handler::{JobHandler, JobHandlerContext},
        tasks::BuildingUpgradeTask,
    },
};

pub struct UpgradeBuildingJobHandler {
    payload: BuildingUpgradeTask,
}

impl UpgradeBuildingJobHandler {
    pub fn new(payload: BuildingUpgradeTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for UpgradeBuildingJobHandler {
    #[instrument(skip_all, fields(
        task_type = "BuildingUpgrade",
        slot_id = self.payload.slot_id,
        name = ?self.payload.building_name,
        village_id = job.village_id,
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Executing BuildingUpgrade job");

        let village_id = job.village_id as u32;
        let village_repo = ctx.uow.villages();
        let mut village = village_repo.get_by_id(village_id).await?;

        village.set_building_level_at_slot(
            self.payload.slot_id,
            self.payload.level,
            ctx.config.speed,
        )?;
        village_repo.save(&village).await?;

        // Update player's total culture points
        update_player_culture_points(&ctx.uow, village.player_id).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::sync::Arc;

    use parabellum_game::test_utils::{
        PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
    };
    use parabellum_types::Result;
    use parabellum_types::{buildings::BuildingName, tribe::Tribe};

    use super::*;
    use crate::{
        config::Config,
        jobs::{Job, JobPayload},
        test_utils::tests::MockUnitOfWork,
        uow::UnitOfWork,
    };

    async fn setup_job_test() -> Result<(
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

        let slot_id = 19;
        village.set_building_level_at_slot(slot_id, 1, config.speed)?;
        let initial_population = village.population;

        let village_id = village.id;
        let player_id = player.id;

        let mock_uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());
        mock_uow.villages().save(&village).await?;

        let payload = BuildingUpgradeTask {
            slot_id,
            building_name: BuildingName::MainBuilding,
            level: 2,
        };
        let job_payload = JobPayload::new("BuildingUpgrade", json!(payload.clone()));
        let job = Job::new(player_id, village_id as i32, 0, job_payload);

        Ok((job, config, mock_uow, village_id, initial_population))
    }

    #[tokio::test]
    async fn test_upgrade_building_job_handler_success() -> Result<()> {
        let (job, config, uow, village_id, initial_population) = setup_job_test().await?;

        let handler =
            UpgradeBuildingJobHandler::new(serde_json::from_value(job.task.data.clone())?);
        let context = JobHandlerContext { uow, config };
        handler.handle(&context, &job).await?;

        let saved_village = context.uow.villages().get_by_id(village_id).await?;
        let building_in_db = saved_village.get_building_by_slot_id(19).unwrap();

        assert_eq!(
            building_in_db.building.level, 2,
            "Expected building upgraded at level {}, got {}",
            2, building_in_db.building.level
        );

        assert_eq!(initial_population, 2, "Wrong initial village population");
        assert_eq!(
            saved_village.population,
            initial_population + 1,
            "Village population not updated correctly"
        );
        Ok(())
    }
}
