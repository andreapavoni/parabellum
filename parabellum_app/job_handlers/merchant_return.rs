use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_types::errors::ApplicationError;

use crate::jobs::{
    Job,
    handler::{JobHandler, JobHandlerContext},
    tasks::MerchantReturnTask,
};

pub struct MerchantReturnJobHandler {
    payload: MerchantReturnTask,
}

impl MerchantReturnJobHandler {
    pub fn new(payload: MerchantReturnTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for MerchantReturnJobHandler {
    #[instrument(skip_all, fields(
        task_type = "MerchantReturn",
        merchants_used = self.payload.merchants_used,
        player_id = %job.player_id,
        village_id = job.village_id,
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        _ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!(
            "Merchants ({}) returned to village {}.",
            self.payload.merchants_used, job.village_id
        );

        // Done. Once the JobWorker sets the job as "Completed", the query `get_busy_merchants_count`
        // won't count the merchants of this jobs as busy, freeing them.
        Ok(())
    }
}
