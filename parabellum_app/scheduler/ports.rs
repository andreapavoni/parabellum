//! Scheduler ports.
//!
//! Scheduler ports hide the operational CQRS/ES worker implementation from the
//! application facade.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parabellum_types::errors::ApplicationError;

/// Port for scheduled action execution.
///
/// Implementations fetch due actions and append canonical workflow facts
/// atomically through the CQRS/ES runtime.
#[async_trait]
pub trait SchedulerPort: Send + Sync {
    /// Processes scheduled actions due at or before `before_or_equal`.
    ///
    /// Returns the number of actions processed in the current pass.
    async fn process_due_actions(
        &self,
        before_or_equal: DateTime<Utc>,
        limit: i64,
    ) -> Result<usize, ApplicationError>;
}
