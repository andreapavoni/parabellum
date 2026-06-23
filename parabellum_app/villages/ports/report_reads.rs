//! Read/context port for report use cases.
//!
//! Report use cases consume projected report read models and keep report
//! lookup ownership out of the broad village query port.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::models::ReportModel;

/// Loads projected report data for application use cases.
#[async_trait]
pub trait ReportReadPort: Send + Sync {
    /// Lists reports visible to one player.
    async fn list_reports_for_player(
        &self,
        player_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ReportModel>, ApplicationError>;

    /// Loads one report visible to one player.
    async fn get_report_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<ReportModel>, ApplicationError>;

    /// Counts unread reports for one player.
    async fn count_unread_reports_for_player(
        &self,
        player_id: Uuid,
    ) -> Result<i64, ApplicationError>;
}
