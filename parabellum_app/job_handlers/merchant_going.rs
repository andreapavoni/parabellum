use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_game::models::village::Village;
use parabellum_types::{
    errors::ApplicationError,
    reports::{MarketplaceDeliveryReportPayload, ReportPayload},
};

use crate::jobs::{
    Job, JobPayload,
    handler::{JobHandler, JobHandlerContext},
    tasks::{MerchantGoingTask, MerchantReturnTask},
};
use crate::repository::{NewReport, ReportAudience};

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

        // Create marketplace delivery report
        let report_repo = ctx.uow.reports();
        let player_repo = ctx.uow.players();

        let origin_village = village_repo
            .get_by_id(self.payload.origin_village_id)
            .await?;
        let sender_player = player_repo.get_by_id(origin_village.player_id).await?;
        let receiver_player = player_repo.get_by_id(destination_village.player_id).await?;

        let delivery_payload = MarketplaceDeliveryReportPayload {
            sender_player: sender_player.username.clone(),
            sender_village: origin_village.name.clone(),
            sender_position: origin_village.position.clone(),
            receiver_player: receiver_player.username.clone(),
            receiver_village: destination_village.name.clone(),
            receiver_position: destination_village.position.clone(),
            resources: self.payload.resources.clone(),
            merchants_used: self.payload.merchants_used,
        };

        let new_report = NewReport {
            report_type: "marketplace_delivery".to_string(),
            payload: ReportPayload::MarketplaceDelivery(delivery_payload),
            actor_player_id: origin_village.player_id,
            actor_village_id: Some(origin_village.id),
            target_player_id: Some(destination_village.player_id),
            target_village_id: Some(destination_village.id),
        };

        let mut audiences = vec![ReportAudience {
            player_id: origin_village.player_id,
            read_at: None,
        }];

        if destination_village.player_id != origin_village.player_id {
            audiences.push(ReportAudience {
                player_id: destination_village.player_id,
                read_at: None,
            });
        }

        report_repo.add(&new_report, &audiences).await?;

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
