use std::{sync::Arc, time::Duration};

use tokio::time;
use tracing::{error, info};

use crate::es::VillageEsService;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Runtime settings for the ES scheduled-action worker.
#[derive(Debug, Clone, Copy)]
pub struct EsScheduledActionWorkerConfig {
    pub batch_limit: i64,
    pub poll_interval: Duration,
}

impl EsScheduledActionWorkerConfig {
    pub fn new(batch_limit: i64) -> Self {
        Self {
            batch_limit: batch_limit.max(1),
            poll_interval: DEFAULT_POLL_INTERVAL,
        }
    }
}

/// Polls and executes due ES scheduled actions.
///
/// The worker owns only ticking and logging. Claiming due actions, executing
/// payloads, and status transitions remain in `VillageEsService`.
#[derive(Debug, Clone)]
pub struct EsScheduledActionWorker {
    service: VillageEsService,
    config: EsScheduledActionWorkerConfig,
}

impl EsScheduledActionWorker {
    pub fn new(service: VillageEsService, batch_limit: i64) -> Self {
        Self::new_with_config(service, EsScheduledActionWorkerConfig::new(batch_limit))
    }

    pub fn new_with_config(
        service: VillageEsService,
        config: EsScheduledActionWorkerConfig,
    ) -> Self {
        Self {
            service,
            config: EsScheduledActionWorkerConfig {
                batch_limit: config.batch_limit.max(1),
                poll_interval: config.poll_interval,
            },
        }
    }

    pub fn run(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(self.config.poll_interval);
            info!(
                batch_limit = self.config.batch_limit,
                poll_interval_ms = self.config.poll_interval.as_millis(),
                "scheduler worker started"
            );

            loop {
                interval.tick().await;
                if let Err(err) = self.process_due_once().await {
                    error!(error = ?err, "ES scheduled action worker tick failed");
                }
            }
        });
    }

    pub async fn process_due_once(&self) -> Result<usize, mini_cqrs_es::CqrsError> {
        let processed = self
            .service
            .process_due_actions(chrono::Utc::now(), self.config.batch_limit)
            .await?;
        if processed > 0 {
            info!(
                action = "scheduler_tick_processed",
                processed,
                batch_limit = self.config.batch_limit,
                "scheduled action worker tick processed actions"
            );
        }
        Ok(processed)
    }
}
