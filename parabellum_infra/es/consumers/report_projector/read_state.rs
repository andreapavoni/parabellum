//! Report read-state projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use sqlx::{Postgres, Transaction};

use crate::es::consumers::report_projector::ReportProjector;

impl ReportProjector {
    pub(super) async fn project_read_state_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::ReportMarkedAsRead { .. } => {
                Some(self.project_report_marked_as_read(tx, event).await)
            }
            _ => None,
        }
    }

    async fn project_report_marked_as_read(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::ReportMarkedAsRead {
            report_id,
            player_id,
            read_at,
        } = event
        else {
            unreachable!("project_report_marked_as_read called with non-ReportMarkedAsRead event");
        };
        let updated = self
            .reports
            .mark_as_read_in_tx(tx, *report_id, *player_id, *read_at)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        if updated {
            return Ok(());
        }
        self.reports
            .mark_latest_unread_as_read_before_in_tx(tx, *player_id, *read_at)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(())
    }
}
