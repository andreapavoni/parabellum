use chrono::{DateTime, Utc};
use mini_cqrs_es::{Command, CqrsError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent};

/// Marks one report as read for a player.
#[derive(Debug, Clone)]
pub struct MarkReportRead {
    /// Report id being marked.
    pub report_id: Uuid,
    /// Player that owns the report audience.
    pub player_id: Uuid,
    /// Timestamp selected by the application clock.
    pub read_at: DateTime<Utc>,
}

impl Command for MarkReportRead {
    type Aggregate = VillageAggregate;

    async fn handle(&self, _aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        Ok(vec![VillageEvent::ReportMarkedAsRead {
            report_id: self.report_id,
            player_id: self.player_id,
            read_at: self.read_at,
        }])
    }
}
