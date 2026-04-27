use uuid::Uuid;

use crate::toasty_time::jiff_to_chrono_utc;
use parabellum_app::repository::{ReportAudience, ReportRecord};
use parabellum_types::{errors::ApplicationError, reports::ReportPayload};

#[derive(Debug, Clone, toasty::Model)]
#[table = "reports"]
pub struct ReportDbRow {
    #[key]
    pub id: Uuid,
    pub report_type: String,

    #[serialize(json)]
    pub payload: ReportPayload,

    #[index]
    pub actor_player_id: Uuid,
    pub actor_village_id: Option<i32>,
    pub target_player_id: Option<Uuid>,
    pub target_village_id: Option<i32>,
    pub created_at: jiff::Timestamp,
}

#[derive(Debug, Clone, toasty::Model)]
#[table = "report_reads"]
pub struct ReportReadDbRow {
    #[key]
    pub report_id: Uuid,
    #[key]
    #[index]
    pub player_id: Uuid,
    pub read_at: Option<jiff::Timestamp>,
}

pub fn to_report_record(
    report: ReportDbRow,
    audience: &ReportAudience,
) -> Result<ReportRecord, ApplicationError> {
    Ok(ReportRecord {
        id: report.id,
        report_type: report.report_type,
        payload: report.payload,
        actor_player_id: report.actor_player_id,
        actor_village_id: report.actor_village_id.map(|id| id as u32),
        target_player_id: report.target_player_id,
        target_village_id: report.target_village_id.map(|id| id as u32),
        created_at: jiff_to_chrono_utc(report.created_at)?,
        read_at: audience.read_at,
    })
}
