use async_trait::async_trait;
use tracing::{info, instrument};

use crate::{
    error::ApplicationError,
    game::models::buildings::Building,
    jobs::{
        Job,
        handler::{JobHandler, JobHandlerContext},
        tasks::AddBuildingTask,
    },
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

        let building = Building::new(self.payload.name.clone()).at_level(1)?;

        village.add_building_at_slot(building, self.payload.slot_id)?;

        village.update_state();
        village_repo.save(&village).await?;

        Ok(())
    }
}
