use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, instrument};

use crate::{
    app::job_handlers::attack::AttackJobHandler,
    jobs::{
        handler::{JobHandler, JobHandlerContext},
        Job, JobTask,
    },
    repository::uow::UnitOfWorkProvider,
};

/// Responsible for polling and execute job.
pub struct JobWorker {
    uow_provider: Arc<dyn UnitOfWorkProvider>,
}

impl JobWorker {
    pub fn new(uow_provider: Arc<dyn UnitOfWorkProvider>) -> Self {
        Self { uow_provider }
    }

    /// Run worker loop inside a tokio task.
    pub fn run(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            info!("Job Worker started.");

            loop {
                interval.tick().await;
                if let Err(e) = self.process_due_jobs().await {
                    error!(error = ?e, "Error while processing job queue");
                }
            }
        });
    }

    #[instrument(skip_all)]
    pub async fn process_due_jobs(&self) -> Result<()> {
        let uow = self.uow_provider.begin().await?;
        let due_jobs = uow.jobs().find_and_lock_due_jobs(10).await?;
        if !due_jobs.is_empty() {
            info!(count = due_jobs.len(), "Processing due jobs");
        }
        self.process_jobs(&due_jobs).await
    }

    #[instrument(skip_all, fields(job_count = jobs.len()))]
    pub async fn process_jobs(&self, jobs: &Vec<Job>) -> Result<()> {
        for job in jobs {
            let uow = self.uow_provider.begin().await?;
            let context = JobHandlerContext { uow };
            let job_id = job.id;
            let job_type = std::any::type_name_of_val(&job.task);

            let span = tracing::info_span!(
                "process_job",
                job_id = %job_id,
                job_type = %job_type,
                player_id = %job.player_id,
                village_id = %job.village_id
            );
            let _enter = span.enter();
            info!("Processing job");

            let handler: Box<dyn JobHandler> = match job.task.clone() {
                JobTask::Attack(payload) => Box::new(AttackJobHandler::new(payload)),
                _ => {
                    error!("No handler found for job type");
                    continue;
                }
            };

            // 3. Execute handler
            let task_result = handler.handle(&context).await;

            // 4. Handle transaction and job state
            match task_result {
                Ok(_) => {
                    context.uow.jobs().mark_as_completed(job_id).await?;
                    context.uow.commit().await?;
                    info!("Job completed successfully");
                }
                Err(e) => {
                    error!(error = ?e, "Job failed");
                    context.uow.rollback().await?;

                    let uow_fail = self.uow_provider.begin().await?;
                    uow_fail
                        .jobs()
                        .mark_as_failed(job_id, &e.to_string())
                        .await?;
                    uow_fail.commit().await?;
                }
            }
        }
        Ok(())
    }
}
