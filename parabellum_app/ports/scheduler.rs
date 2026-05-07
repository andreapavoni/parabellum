use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parabellum_types::errors::ApplicationError;

#[async_trait]
pub trait SchedulerPort: Send + Sync {
    async fn process_due_actions(
        &self,
        before_or_equal: DateTime<Utc>,
        limit: i64,
    ) -> Result<usize, ApplicationError>;
}
