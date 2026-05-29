use chrono::Utc;
use mini_cqrs_es::{Command, CqrsError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent};

#[derive(Debug, Clone)]
pub struct MarkReportRead {
    pub report_id: Uuid,
    pub player_id: Uuid,
}

impl Command for MarkReportRead {
    type Aggregate = VillageAggregate;

    async fn handle(&self, _aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        Ok(vec![VillageEvent::ReportMarkedAsRead {
            report_id: self.report_id,
            player_id: self.player_id,
            read_at: Utc::now(),
        }])
    }
}
