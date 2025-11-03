use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, instrument};

use crate::{
    Result,
    error::ApplicationError,
    jobs::{
        Job,
        handler::{JobHandler, JobHandlerContext},
        tasks::ResearchAcademyTask,
    },
    repository::VillageRepository,
};

pub struct ResearchAcademyJobHandler {
    payload: ResearchAcademyTask,
}

impl ResearchAcademyJobHandler {
    pub fn new(payload: ResearchAcademyTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for ResearchAcademyJobHandler {
    #[instrument(skip_all, fields(
        task_type = "ResearchAcademy",
        unit = ?self.payload.unit,
        village_id = job.village_id
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Executing ResearchAcademy job");

        let village_repo: Arc<dyn VillageRepository + '_> = ctx.uow.villages();
        let village_id = job.village_id as u32;

        let mut village = village_repo.get_by_id(village_id).await?;

        village.research_academy(self.payload.unit.clone())?;
        village_repo.save(&village).await?;

        info!(unit = ?self.payload.unit, "Unit research completed.");
        Ok(())
    }
}
