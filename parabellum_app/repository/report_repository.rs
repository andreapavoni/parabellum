use chrono::{DateTime, Utc};
use parabellum_types::reports::ReportPayload;
use uuid::Uuid;

use parabellum_types::errors::ApplicationError;

#[derive(Debug, Clone)]
pub struct NewReport {
    pub report_type: String,
    pub payload: ReportPayload,
    pub actor_player_id: Uuid,
    pub actor_village_id: Option<u32>,
    pub target_player_id: Option<Uuid>,
    pub target_village_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ReportAudience {
    pub player_id: Uuid,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ReportRecord {
    pub id: Uuid,
    pub report_type: String,
    pub payload: ReportPayload,
    pub actor_player_id: Uuid,
    pub actor_village_id: Option<u32>,
    pub target_player_id: Option<Uuid>,
    pub target_village_id: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

#[async_trait::async_trait]
pub trait ReportRepository: Send + Sync {
    async fn add(
        &self,
        report: &NewReport,
        audiences: &[ReportAudience],
    ) -> Result<(), ApplicationError>;

    async fn list_for_player(
        &self,
        player_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ReportRecord>, ApplicationError>;

    async fn mark_as_read(&self, report_id: Uuid, player_id: Uuid) -> Result<(), ApplicationError>;
}
