use crate::{
    jobs::{
        handler::{JobHandler, JobHandlerContext},
        tasks::ResearchSmithyTask,
        Job,
    },
    repository::VillageRepository,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, instrument};

pub struct ResearchSmithyJobHandler {
    payload: ResearchSmithyTask,
}

impl ResearchSmithyJobHandler {
    pub fn new(payload: ResearchSmithyTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for ResearchSmithyJobHandler {
    #[instrument(skip_all, fields(
        task_type = "ResearchSmithy",
        unit = ?self.payload.unit,
        village_id = job.village_id
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<()> {
        info!("Executing ResearchSmithy job");

        let village_repo: Arc<dyn VillageRepository + '_> = ctx.uow.villages();
        let village_id = job.village_id as u32;

        let mut village = village_repo
            .get_by_id(village_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Village not found"))?;

        village.upgrade_smithy(self.payload.unit.clone())?;
        village_repo.save(&village).await?;

        info!(unit = ?self.payload.unit, "Smithy upgrade completed.");
        Ok(())
    }
}
