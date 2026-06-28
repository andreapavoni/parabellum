//! Typed rows for report projections.

use parabellum_app::villages::models::ReportModel;
use parabellum_types::errors::ApplicationError;
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub(super) struct DbReportRow {
    id: Uuid,
    report_type: String,
    payload: Json<serde_json::Value>,
    actor_player_id: Uuid,
    actor_village_id: Option<i32>,
    target_player_id: Option<Uuid>,
    target_village_id: Option<i32>,
    created_at: chrono::DateTime<chrono::Utc>,
    read_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl TryFrom<DbReportRow> for ReportModel {
    type Error = ApplicationError;

    fn try_from(row: DbReportRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            report_type: row.report_type,
            payload: serde_json::from_value(row.payload.0)?,
            actor_player_id: row.actor_player_id,
            actor_village_id: row.actor_village_id.map(|v| v as u32),
            target_player_id: row.target_player_id,
            target_village_id: row.target_village_id.map(|v| v as u32),
            created_at: row.created_at,
            read_at: row.read_at,
        })
    }
}
