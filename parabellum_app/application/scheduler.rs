use chrono::{DateTime, Utc};
use parabellum_types::errors::ApplicationError;

use super::GameApplication;

pub async fn process_due_actions(
    app: &GameApplication,
    before_or_equal: DateTime<Utc>,
    limit: i64,
) -> Result<usize, ApplicationError> {
    app.scheduler_port()
        .process_due_actions(before_or_equal, limit)
        .await
}
