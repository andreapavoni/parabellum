use crate::jobs::Job;
use anyhow::Result;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait JobRepository: Send + Sync {
    /// Creates a new job on the db.
    async fn add(&self, job: &Job) -> Result<()>;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Job>>;

    async fn list_by_player_id(&self, id: Uuid) -> Result<Vec<Job>>;

    /// Finds and locks atomically overdue jobs, setting the status to "Processing".
    /// This prevents several workers getting the  same job.
    async fn find_and_lock_due_jobs(&self, limit: i64) -> Result<Vec<Job>>;

    /// Set job status to "Completed".
    async fn mark_as_completed(&self, job_id: Uuid) -> Result<()>;

    /// Set job status to "Failed", possibly with an error message.
    async fn mark_as_failed(&self, job_id: Uuid, error_message: &str) -> Result<()>;
}
