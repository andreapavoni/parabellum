use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_game::models::village::Village;
use parabellum_types::errors::ApplicationError;

use crate::jobs::{
    Job, JobPayload,
    handler::{JobHandler, JobHandlerContext},
    tasks::{MerchantGoingTask, MerchantReturnTask},
};

pub struct MerchantGoingJobHandler {
    payload: MerchantGoingTask,
}

impl MerchantGoingJobHandler {
    pub fn new(payload: MerchantGoingTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for MerchantGoingJobHandler {
    #[instrument(skip_all, fields(
        task_type = "MerchantGoing",
        merchants_used = self.payload.merchants_used,
        player_id = %job.player_id,
        origin_village_id = job.village_id,
        destination_village_id = self.payload.destination_village_id,
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!(
            "Merchants ({}) from village {} are arriving at village {}.",
            self.payload.merchants_used, job.village_id, self.payload.destination_village_id
        );

        let village_repo = ctx.uow.villages();

        let mut destination_village: Village = village_repo
            .get_by_id(self.payload.destination_village_id)
            .await?;

        destination_village.store_resources(&self.payload.resources);

        village_repo.save(&destination_village).await?;

        info!(
            "Resources successfully delivered to village {}.",
            self.payload.destination_village_id
        );

        let return_payload = MerchantReturnTask {
            destination_village_id: self.payload.origin_village_id,
            origin_village_id: self.payload.destination_village_id,
            merchants_used: self.payload.merchants_used,
        };
        let return_job_payload =
            JobPayload::new("MerchantReturn", serde_json::to_value(&return_payload)?);

        let return_job = Job::new(
            job.player_id,
            job.village_id,
            self.payload.travel_time_secs,
            return_job_payload,
        );

        ctx.uow.jobs().add(&return_job).await?;

        info!(
            return_job_id = %return_job.id,
            arrival_at = %return_job.completed_at,
            "Merchant return job planned."
        );

        Ok(())
    }
}
