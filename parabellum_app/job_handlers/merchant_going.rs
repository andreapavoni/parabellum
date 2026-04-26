use async_trait::async_trait;
use chrono::Duration;
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

        let return_arrival_at = job.completed_at + Duration::seconds(self.payload.travel_time_secs);
        let return_job = Job::with_deadline(
            job.player_id,
            job.village_id,
            return_job_payload,
            return_arrival_at,
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

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use parabellum_game::test_utils::{
        PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
    };
    use parabellum_types::{Result, common::ResourceGroup, tribe::Tribe};

    use super::*;
    use crate::{
        config::Config,
        jobs::JobPayload,
        test_utils::tests::{MockUnitOfWork, set_village_resources},
        uow::UnitOfWork,
    };

    async fn setup_test() -> Result<(
        Box<dyn UnitOfWork<'static> + 'static>,
        parabellum_game::models::village::Village,
        parabellum_game::models::village::Village,
        uuid::Uuid,
    )> {
        let mock_uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());
        let sender_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        let receiver_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        let sender_village = village_factory(VillageFactoryOptions {
            player: Some(sender_player.clone()),
            ..Default::default()
        });
        let mut receiver_village = village_factory(VillageFactoryOptions {
            player: Some(receiver_player.clone()),
            ..Default::default()
        });
        set_village_resources(&mut receiver_village, ResourceGroup::default());

        mock_uow.players().save(&sender_player).await?;
        mock_uow.players().save(&receiver_player).await?;
        mock_uow.villages().save(&sender_village).await?;
        mock_uow.villages().save(&receiver_village).await?;

        Ok((mock_uow, sender_village, receiver_village, sender_player.id))
    }

    #[tokio::test]
    async fn merchant_return_is_scheduled_from_original_arrival_timestamp() -> Result<()> {
        let (uow, sender_village, receiver_village, player_id) = setup_test().await?;
        let config = std::sync::Arc::new(Config::from_env());
        let travel = 120_i64;
        let payload = MerchantGoingTask {
            resources: ResourceGroup(200, 50, 0, 0),
            origin_village_id: sender_village.id,
            destination_village_id: receiver_village.id,
            merchants_used: 2,
            travel_time_secs: travel,
        };
        let handler = MerchantGoingJobHandler::new(payload.clone());
        let job_payload = JobPayload::new("MerchantGoing", serde_json::to_value(&payload)?);
        let outbound_arrival = Utc::now() - Duration::seconds(300);
        let job = Job::with_deadline(
            player_id,
            sender_village.id as i32,
            job_payload,
            outbound_arrival,
        );
        let ctx = crate::jobs::handler::JobHandlerContext { uow, config };

        handler.handle(&ctx, &job).await?;

        let updated_receiver = ctx.uow.villages().get_by_id(receiver_village.id).await?;
        assert_eq!(updated_receiver.stored_resources().lumber(), 200);
        assert_eq!(updated_receiver.stored_resources().clay(), 50);

        let jobs = ctx.uow.jobs().list_by_player_id(player_id).await?;
        assert_eq!(jobs.len(), 1);
        let return_job = &jobs[0];
        assert_eq!(return_job.task.task_type, "MerchantReturn");
        assert_eq!(
            return_job.completed_at,
            outbound_arrival + Duration::seconds(travel),
            "Return should be anchored to original outbound arrival, not current wall time"
        );
        assert!(
            return_job.completed_at <= Utc::now(),
            "With enough downtime the return leg should already be due"
        );

        Ok(())
    }
}
