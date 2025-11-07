use async_trait::async_trait;
use parabellum_core::ApplicationError;
use parabellum_game::models::buildings::Building;
use tracing::{info, instrument};

use crate::jobs::{
    Job,
    handler::{JobHandler, JobHandlerContext},
    tasks::AddBuildingTask,
};

pub struct AddBuildingJobHandler {
    payload: AddBuildingTask,
}

impl AddBuildingJobHandler {
    pub fn new(payload: AddBuildingTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for AddBuildingJobHandler {
    #[instrument(skip_all, fields(
        task_type = "AddBuilding",
        slot_id = ?job.task.data.get("slot_id"),
                name = ?job.task.data.get("name"),
        player_id = %job.player_id,
        village_id = job.village_id,
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Executing AddBuilding job");

        let village_id = job.village_id as u32;
        let village_repo = ctx.uow.villages();
        let mut village = village_repo.get_by_id(village_id).await?;

        let building = Building::new(self.payload.name.clone(), ctx.config.speed)
            .at_level(1, ctx.config.speed)?;

        village.add_building_at_slot(building, self.payload.slot_id)?;

        village.update_state();
        village_repo.save(&village).await?;

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
    use parabellum_types::{buildings::BuildingName, tribe::Tribe};

    use super::*;
    use crate::{
        config::Config,
        jobs::{Job, JobPayload},
        test_utils::tests::MockUnitOfWork,
        uow::UnitOfWork,
    };

    #[tokio::test]
    async fn test_add_building_job_handler_success() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let config = Arc::new(Config::from_env());

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        let village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });

        let village_id = village.id;
        let player_id = player.id;
        mock_uow.villages().save(&village).await.unwrap();

        let slot_id_to_build: u8 = 22;
        let building_to_build = BuildingName::Warehouse;

        let payload = AddBuildingTask {
            slot_id: slot_id_to_build,
            name: building_to_build.clone(),
            village_id: village_id as i32,
        };
        let job_payload = JobPayload::new("AddBuilding", json!(payload));
        let job = Job::new(player_id, village_id as i32, 0, job_payload);

        let handler = AddBuildingJobHandler::new(payload);

        let context = JobHandlerContext {
            uow: mock_uow,
            config,
        };

        let result = handler.handle(&context, &job).await;

        assert!(
            result.is_ok(),
            "Job handler should succeed: {:?}",
            result.err()
        );

        // Check if building was added
        let saved_village = context.uow.villages().get_by_id(village_id).await.unwrap();
        let new_building = saved_village.get_building_by_slot_id(slot_id_to_build);

        assert!(
            new_building.is_some(),
            "Building should be added to the village"
        );
        let new_building = new_building.unwrap().building;
        assert_eq!(
            new_building.name, building_to_build,
            "Correct building type should be added"
        );
        assert_eq!(new_building.level, 1, "Building should be at level 1");

        // Check if population was updated
        assert!(
            saved_village.population > village.population,
            "Population should increase after building"
        );
    }
}
