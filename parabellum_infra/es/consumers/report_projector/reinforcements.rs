//! Reinforcement report projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::projection_repositories::ReportKind;
use parabellum_game::models::army::Army;
use parabellum_types::reports::{ReinforcementReportPayload, ReportPayload};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::es::consumers::report_projector::{
    ReportProjection, ReportProjector, SourceTargetReportContext,
};

impl ReportProjector {
    pub(super) async fn project_reinforcement_report_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        projected_report_id: Uuid,
        event: &VillageEvent,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::ReinforcementArrived { .. } => Some(
                self.project_reinforcement_arrived(tx, projected_report_id, event)
                    .await,
            ),
            _ => None,
        }
    }

    async fn project_reinforcement_arrived(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        projected_report_id: Uuid,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::ReinforcementArrived {
            player_id,
            source_village_id,
            target_village_id,
            army,
            ..
        } = event
        else {
            unreachable!(
                "project_reinforcement_arrived called with non-ReinforcementArrived event"
            );
        };
        let Some(context) = self
            .source_target_context_in_tx(tx, *source_village_id, *target_village_id)
            .await?
        else {
            return Ok(());
        };
        let payload = reinforcement_payload(&context, army);
        let audiences = Self::audience_with_target(*player_id, context.target.player_id);
        self.project_report_in_tx(
            tx,
            ReportProjection::source_target(
                projected_report_id,
                ReportKind::Reinforcement,
                payload,
                &context,
                *source_village_id,
                *target_village_id,
                audiences,
            ),
        )
        .await
    }
}

fn reinforcement_payload(context: &SourceTargetReportContext, army: &Army) -> ReportPayload {
    ReportPayload::Reinforcement(ReinforcementReportPayload {
        sender_player: context.source_player.clone(),
        sender_village: context.source.village_name.clone(),
        sender_position: context.source.position.clone(),
        receiver_player: context.target_player.clone(),
        receiver_village: context.target.village_name.clone(),
        receiver_position: context.target.position.clone(),
        tribe: army.tribe.clone(),
        units: army.units().clone(),
        has_hero: army.hero().is_some(),
    })
}
