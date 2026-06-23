//! Projected report models.

use chrono::{DateTime, Utc};
use parabellum_types::reports::ReportPayload;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Projected report row visible to a player audience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportModel {
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
