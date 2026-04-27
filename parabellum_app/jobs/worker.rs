use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, instrument};

use parabellum_types::Result;

use crate::{
    config::Config,
    jobs::{
        Job,
        handler::{JobHandler, JobHandlerContext, JobRegistry},
    },
    uow::UnitOfWorkProvider,
};

/// Responsible for polling and execute job.
pub struct JobWorker {
    uow_provider: Arc<dyn UnitOfWorkProvider>,
    registry: Arc<dyn JobRegistry>,
    config: Arc<Config>,
}

impl JobWorker {
    pub fn new(
        uow_provider: Arc<dyn UnitOfWorkProvider>,
        registry: Arc<dyn JobRegistry>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            uow_provider,
            registry,
            config,
        }
    }

    /// Start job worker process.
    pub fn run(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            info!("Job Worker started.");

            loop {
                interval.tick().await;
                if let Err(e) = self.process_due_jobs().await {
                    error!(error = ?e.to_string(), "Error while processing job queue");
                }
            }
        });
    }

    #[instrument(skip_all)]
    /// Process due jobs.
    pub async fn process_due_jobs(&self) -> Result<()> {
        let uow = self.uow_provider.tx().await?;
        let transactional = uow.is_transactional();
        let mut due_jobs = uow.jobs().find_and_lock_due_jobs(10).await?;
        if transactional {
            uow.commit().await?;
        }
        due_jobs.sort_by(|a, b| a.completed_at.cmp(&b.completed_at));
        if !due_jobs.is_empty() {
            info!(count = due_jobs.len(), "Processing due jobs");
        }
        self.process_jobs(&due_jobs).await
    }

    #[instrument(skip_all, fields(job_count = jobs.len()))]
    /// Process given jobs.
    pub async fn process_jobs(&self, jobs: &Vec<Job>) -> Result<()> {
        for job in jobs {
            let uow = self.uow_provider.tx().await?;
            let context = JobHandlerContext {
                uow,
                config: self.config.clone(),
            };
            let transactional = context.uow.is_transactional();
            let job_id = job.id;
            let job_type = job.task.task_type.clone();

            let span = tracing::info_span!(
                "process_job",
                job_id = %job_id,
                job_type = %job_type,
                player_id = %job.player_id,
                village_id = %job.village_id
            );
            let _enter = span.enter();
            info!("Processing job");

            let handler: Box<dyn JobHandler> = match self
                .registry
                .get_handler(&job_type, &job.task.data)
            {
                Ok(handler) => handler,
                Err(e) => {
                    error!(job_id = %job.id, error = ?e.to_string(), "Failed to create handler for job");
                    // This error could be due to deserialization
                    // or an unregistered task_type.
                    // Mark the job as failed and continue.
                    if transactional {
                        context.uow.rollback().await?;
                    }
                    let uow_fail = self.uow_provider.tx().await?;
                    let fail_transactional = uow_fail.is_transactional();
                    uow_fail
                        .jobs()
                        .mark_as_failed(job_id, &e.to_string())
                        .await?;
                    if fail_transactional {
                        uow_fail.commit().await?;
                    }
                    continue; // Go to the next job
                }
            };
            let task_result = handler.handle(&context, job).await;

            match task_result {
                Ok(_) => {
                    let current_job = context.uow.jobs().get_by_id(job_id).await?;
                    let was_rescheduled =
                        matches!(current_job.status, crate::jobs::JobStatus::Pending)
                            && (current_job.updated_at > job.updated_at
                                || current_job.completed_at != job.completed_at
                                || current_job.task.task_type != job.task.task_type
                                || current_job.task.data != job.task.data);
                    if matches!(current_job.status, crate::jobs::JobStatus::Processing)
                        || (matches!(current_job.status, crate::jobs::JobStatus::Pending)
                            && !was_rescheduled)
                    {
                        context.uow.jobs().mark_as_completed(job_id).await?;
                    }
                    if transactional {
                        context.uow.commit().await?;
                    }
                    info!(job_id = %job.id, "Job completed successfully");
                }
                Err(e) => {
                    error!(job_id = %job.id, error = ?e.to_string(), "Job failed");
                    if transactional {
                        context.uow.rollback().await?;
                    }

                    let uow_fail = self.uow_provider.tx().await?;
                    let fail_transactional = uow_fail.is_transactional();
                    uow_fail
                        .jobs()
                        .mark_as_failed(job_id, &e.to_string())
                        .await?;
                    if fail_transactional {
                        uow_fail.commit().await?;
                    }
                }
            }
        }
        Ok(())
    }
}
