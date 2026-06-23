//! Scheduler request models.

use chrono::{DateTime, Utc};

/// Request to process scheduled actions due by a point in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessDueActionsRequest {
    pub before_or_equal: DateTime<Utc>,
    pub limit: i64,
}
