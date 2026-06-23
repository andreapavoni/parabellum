//! Marketplace delivery report projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::projection_repositories::ProjectedReport;
use parabellum_types::common::ResourceGroup;
use parabellum_types::reports::{MarketplaceDeliveryReportPayload, ReportPayload};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::es::consumers::report_projector::{ReportProjector, SourceTargetReportContext};

impl ReportProjector {
    pub(super) async fn project_marketplace_report_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        projected_report_id: Uuid,
        event: &VillageEvent,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::MerchantsArrived { .. } => Some(
                self.project_merchants_arrived(tx, projected_report_id, event)
                    .await,
            ),
            _ => None,
        }
    }

    async fn project_merchants_arrived(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        projected_report_id: Uuid,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::MerchantsArrived {
            player_id,
            source_village_id,
            target_village_id,
            resources,
            merchants_used,
            ..
        } = event
        else {
            unreachable!("project_merchants_arrived called with non-MerchantsArrived event");
        };
        let Some(context) = self
            .source_target_context_in_tx(tx, *source_village_id, *target_village_id)
            .await?
        else {
            return Ok(());
        };
        let payload = marketplace_delivery_payload(&context, resources, *merchants_used);
        let audiences = Self::audience_with_target(*player_id, context.target.player_id);
        self.reports
            .add_projected_in_tx(
                tx,
                &ProjectedReport {
                    id: projected_report_id,
                    report_type: "marketplace_delivery".to_string(),
                    payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                    actor_player_id: context.source.player_id,
                    actor_village_id: Some(*source_village_id),
                    target_player_id: Some(context.target.player_id),
                    target_village_id: Some(*target_village_id),
                },
                &audiences,
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(())
    }
}

fn marketplace_delivery_payload(
    context: &SourceTargetReportContext,
    resources: &ResourceGroup,
    merchants_used: u8,
) -> ReportPayload {
    ReportPayload::MarketplaceDelivery(MarketplaceDeliveryReportPayload {
        sender_player: context.source_player.clone(),
        sender_village: context.source.village_name.clone(),
        sender_position: context.source.position.clone(),
        receiver_player: context.target_player.clone(),
        receiver_village: context.target.village_name.clone(),
        receiver_position: context.target.position.clone(),
        resources: resources.clone(),
        merchants_used,
    })
}
