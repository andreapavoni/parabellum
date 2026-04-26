use uuid::Uuid;

use parabellum_app::repository::{ReportAudience, ReportRecord};
use parabellum_types::{
    errors::{ApplicationError, DbError},
    reports::ReportPayload,
};

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

pub fn chrono_to_jiff(
    value: chrono::DateTime<chrono::Utc>,
) -> Result<jiff::Timestamp, ApplicationError> {
    jiff::Timestamp::from_second(value.timestamp())
        .and_then(|ts| {
            ts.checked_add(jiff::SignedDuration::new(
                0,
                value.timestamp_subsec_nanos() as i32,
            ))
        })
        .map_err(|err| {
            ApplicationError::Db(DbError::Transaction(format!(
                "could not convert chrono datetime to jiff timestamp: {err}"
            )))
        })
}

pub fn jiff_to_chrono_utc(
    value: jiff::Timestamp,
) -> Result<chrono::DateTime<chrono::Utc>, ApplicationError> {
    let nanos_i128 = value.as_nanosecond();
    let nanos_i64 = i64::try_from(nanos_i128).map_err(|_| {
        ApplicationError::Db(DbError::Transaction(
            "jiff timestamp is outside chrono nanosecond range".to_string(),
        ))
    })?;

    Ok(chrono::DateTime::<chrono::Utc>::from_timestamp_nanos(
        nanos_i64,
    ))
}
