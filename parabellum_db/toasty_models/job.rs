use uuid::Uuid;

use crate::toasty_time::{chrono_to_jiff_utc, jiff_to_chrono_utc};
use parabellum_app::jobs::{Job, JobPayload, JobStatus};
use parabellum_types::errors::{ApplicationError, DbError};

#[derive(Debug, Clone, toasty::Model)]
#[table = "jobs"]
pub struct JobRecord {
    #[key]
    pub id: Uuid,

    #[index]
    pub player_id: Uuid,

    #[index]
    pub village_id: i32,

    #[serialize(json)]
    pub task: JobPayload,

    pub status: String,

    pub completed_at: jiff::Timestamp,
    pub created_at: jiff::Timestamp,
    pub updated_at: jiff::Timestamp,
}

impl TryFrom<JobRecord> for Job {
    type Error = ApplicationError;

    fn try_from(record: JobRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            id: record.id,
            player_id: record.player_id,
            village_id: record.village_id,
            task: record.task,
            status: parse_job_status(&record.status)?,
            completed_at: jiff_to_chrono_utc(record.completed_at)?,
            created_at: jiff_to_chrono_utc(record.created_at)?,
            updated_at: jiff_to_chrono_utc(record.updated_at)?,
        })
    }
}

impl TryFrom<&Job> for JobRecord {
    type Error = ApplicationError;

    fn try_from(job: &Job) -> Result<Self, Self::Error> {
        Ok(Self {
            id: job.id,
            player_id: job.player_id,
            village_id: job.village_id,
            task: job.task.clone(),
            status: format_job_status(&job.status).to_string(),
            completed_at: chrono_to_jiff_utc(job.completed_at)?,
            created_at: chrono_to_jiff_utc(job.created_at)?,
            updated_at: chrono_to_jiff_utc(job.updated_at)?,
        })
    }
}

fn parse_job_status(value: &str) -> Result<JobStatus, ApplicationError> {
    match value {
        "Pending" => Ok(JobStatus::Pending),
        "Processing" => Ok(JobStatus::Processing),
        "Completed" => Ok(JobStatus::Completed),
        "Failed" => Ok(JobStatus::Failed),
        _ => Err(ApplicationError::Db(DbError::Transaction(format!(
            "invalid job status value: {value}"
        )))),
    }
}

fn format_job_status(status: &JobStatus) -> &'static str {
    match status {
        JobStatus::Pending => "Pending",
        JobStatus::Processing => "Processing",
        JobStatus::Completed => "Completed",
        JobStatus::Failed => "Failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    #[test]
    fn job_roundtrip_conversion_preserves_core_fields() {
        let now = Utc::now();
        let job = Job {
            id: Uuid::new_v4(),
            player_id: Uuid::new_v4(),
            village_id: 7,
            task: JobPayload::new("Attack", serde_json::json!({"target_village_id": 42})),
            status: JobStatus::Processing,
            completed_at: now + Duration::minutes(15),
            created_at: now,
            updated_at: now + Duration::seconds(30),
        };

        let record = JobRecord::try_from(&job).expect("conversion to toasty record should work");
        let decoded = Job::try_from(record).expect("conversion from toasty record should work");

        assert_eq!(decoded.id, job.id);
        assert_eq!(decoded.player_id, job.player_id);
        assert_eq!(decoded.village_id, job.village_id);
        assert_eq!(decoded.task.task_type, job.task.task_type);
        assert_eq!(decoded.status, job.status);
        assert_eq!(
            decoded.completed_at.timestamp(),
            job.completed_at.timestamp(),
            "roundtrip should preserve second precision for scheduling"
        );
    }
}
