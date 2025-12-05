use uuid::Uuid;

use parabellum_types::errors::ApplicationError;

use crate::jobs::Job;

#[async_trait::async_trait]
pub trait JobRepository: Send + Sync {
    /// Creates a new job on the db.
    async fn add(&self, job: &Job) -> Result<(), ApplicationError>;

    /// Find a job by id.
    async fn get_by_id(&self, id: Uuid) -> Result<Job, ApplicationError>;

    /// Lists jobs created by a player.
    async fn list_by_player_id(&self, id: Uuid) -> Result<Vec<Job>, ApplicationError>;

    /// Lists all pending/processing jobs for a village regardless of task type.
    async fn list_active_jobs_by_village(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError>;

    /// Lists pending/processing building-related jobs for a village.
    async fn list_village_building_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError>;

    /// Lists pending/processing train-unit jobs for a village.
    async fn list_village_training_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError>;

    /// Lists pending/processing academy research jobs for a village.
    async fn list_village_academy_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError>;

    /// Lists pending/processing smithy upgrade jobs for a village.
    async fn list_village_smithy_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError>;

    /// Finds and locks atomically overdue jobs, setting the status to "Processing".
    /// This prevents several workers getting the  same job.
    async fn find_and_lock_due_jobs(&self, limit: i64) -> Result<Vec<Job>, ApplicationError>;

    /// Set job status to "Completed".
    async fn mark_as_completed(&self, job_id: Uuid) -> Result<(), ApplicationError>;

    /// Set job status to "Failed", possibly with an error message.
    async fn mark_as_failed(
        &self,
        job_id: Uuid,
        error_message: &str,
    ) -> Result<(), ApplicationError>;
}
