//! Scheduler use cases.
//!
//! This service owns app-facing scheduler operations and delegates operational
//! execution to infrastructure through `SchedulerPort`.

use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::scheduler::{ports::SchedulerPort, requests::ProcessDueActionsRequest};

/// Application service for scheduled action processing.
#[derive(Clone)]
pub struct SchedulerUseCases {
    scheduler: Arc<dyn SchedulerPort>,
}

impl SchedulerUseCases {
    /// Creates scheduler use cases from the scheduler execution port.
    pub fn new(scheduler: Arc<dyn SchedulerPort>) -> Self {
        Self { scheduler }
    }

    /// Processes due scheduled actions.
    pub async fn process_due_actions(
        &self,
        request: ProcessDueActionsRequest,
    ) -> Result<usize, ApplicationError> {
        self.scheduler
            .process_due_actions(request.before_or_equal, request.limit)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use parabellum_types::errors::ApplicationError;

    use crate::scheduler::{
        ports::SchedulerPort, requests::ProcessDueActionsRequest, use_cases::SchedulerUseCases,
    };

    #[derive(Default)]
    struct FakeScheduler {
        seen: Mutex<Vec<ProcessDueActionsRequest>>,
    }

    #[async_trait]
    impl SchedulerPort for FakeScheduler {
        async fn process_due_actions(
            &self,
            before_or_equal: chrono::DateTime<Utc>,
            limit: i64,
        ) -> Result<usize, ApplicationError> {
            self.seen
                .lock()
                .expect("seen lock should not be poisoned")
                .push(ProcessDueActionsRequest {
                    before_or_equal,
                    limit,
                });
            Ok(7)
        }
    }

    #[tokio::test]
    async fn process_due_actions_delegates_to_scheduler_port() {
        let scheduler = Arc::new(FakeScheduler::default());
        let use_cases = SchedulerUseCases::new(scheduler.clone());
        let before_or_equal = Utc.with_ymd_and_hms(2026, 6, 21, 12, 0, 0).unwrap();

        let processed = use_cases
            .process_due_actions(ProcessDueActionsRequest {
                before_or_equal,
                limit: 100,
            })
            .await
            .unwrap();

        assert_eq!(processed, 7);
        assert_eq!(
            scheduler.seen.lock().unwrap().as_slice(),
            &[ProcessDueActionsRequest {
                before_or_equal,
                limit: 100,
            }]
        );
    }
}
