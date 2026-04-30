use std::sync::Arc;

use mini_cqrs_es::{CqrsError, Query};

use crate::villages::models::{ScheduledActionStatus, ScheduledActionType};
use crate::villages::repositories::ScheduledActionRepository;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScheduledActionStatusCounts {
    /// Number of actions currently pending.
    pub pending: usize,
    /// Number of actions currently locked/processing.
    pub processing: usize,
    /// Number of actions completed successfully.
    pub completed: usize,
    /// Number of actions failed.
    pub failed: usize,
}

/// Query that computes scheduled-action status counters for one village and action type.
pub struct GetScheduledActionStatusCounts {
    pub repository: Arc<dyn ScheduledActionRepository>,
    pub village_id: u32,
    pub action_type: ScheduledActionType,
    /// Optional status filter. When set, only actions with this status are counted.
    pub status_filter: Option<ScheduledActionStatus>,
}

impl Query for GetScheduledActionStatusCounts {
    type Output = Result<ScheduledActionStatusCounts, CqrsError>;

    async fn apply(&self) -> Self::Output {
        let actions = self
            .repository
            .list_by_village_and_type(self.village_id, self.action_type)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let mut counts = ScheduledActionStatusCounts::default();
        for action in actions {
            if let Some(status_filter) = self.status_filter {
                if action.status != status_filter {
                    continue;
                }
            }
            match action.status {
                ScheduledActionStatus::Pending => counts.pending += 1,
                ScheduledActionStatus::Processing => counts.processing += 1,
                ScheduledActionStatus::Completed => counts.completed += 1,
                ScheduledActionStatus::Failed => counts.failed += 1,
            }
        }
        Ok(counts)
    }
}
