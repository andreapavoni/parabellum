//! Report projection repository contracts.

use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::models::ReportModel;

/// Report projection row before audience materialization.
#[derive(Debug, Clone)]
pub struct ProjectedReport {
    pub id: Uuid,
    pub report_type: String,
    pub payload: serde_json::Value,
    pub actor_player_id: Uuid,
    pub actor_village_id: Option<u32>,
    pub target_player_id: Option<Uuid>,
    pub target_village_id: Option<u32>,
}

/// Persistence boundary for projected reports and report audiences.
#[async_trait::async_trait]
pub trait ReportRepository: Send + Sync {
    async fn add_projected(
        &self,
        report: &ProjectedReport,
        audience_player_ids: &[Uuid],
    ) -> Result<(), ApplicationError>;

    async fn list_for_player(
        &self,
        player_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ReportModel>, ApplicationError>;

    async fn get_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<ReportModel>, ApplicationError>;

    async fn count_unread_for_player(&self, player_id: Uuid) -> Result<i64, ApplicationError>;

    async fn mark_as_read(&self, report_id: Uuid, player_id: Uuid) -> Result<(), ApplicationError>;
}
