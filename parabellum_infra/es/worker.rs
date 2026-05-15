use std::{sync::Arc, time::Duration};

use tokio::time;
use tracing::{error, info};

use crate::es::VillageEsService;

/// Polls and executes due ES scheduled actions.
#[derive(Debug, Clone)]
pub struct EsScheduledActionWorker {
    service: VillageEsService,
    batch_limit: i64,
}

impl EsScheduledActionWorker {
    pub fn new(service: VillageEsService, batch_limit: i64) -> Self {
        Self {
            service,
            batch_limit: batch_limit.max(1),
        }
    }

    pub fn run(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            info!("ES scheduled action worker started.");

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
            .process_due_actions(chrono::Utc::now(), self.batch_limit)
            .await?;
        if processed > 0 {
            info!(
                action = "scheduler_tick_processed",
                processed,
                batch_limit = self.batch_limit,
                "scheduled action worker tick processed actions"
            );
        }
        Ok(processed)
    }
}
