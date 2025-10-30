use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

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
            println!("Job Worker started.");

            loop {
                interval.tick().await;
                if let Err(e) = self.process_due_jobs().await {
                    eprintln!("Error while processing job: {}", e);
                }
            }
        });
    }

    pub async fn process_due_jobs(&self) -> Result<()> {
        let uow = self.uow_provider.begin().await?;
        let due_jobs = uow.jobs().find_and_lock_due_jobs(10).await?;
        self.process_jobs(&due_jobs).await
    }

    pub async fn process_jobs(&self, jobs: &Vec<Job>) -> Result<()> {
        for job in jobs {
            // Begin UoW for this job
            let uow = self.uow_provider.begin().await?;

            // 2. Context now includes UoW
            let context = JobHandlerContext { uow };
            let job_id = job.id;

            // --- Dispatch ---
            let handler: Box<dyn JobHandler> = match job.task.clone() {
                JobTask::Attack(payload) => Box::new(AttackJobHandler::new(payload)),
                _ => continue,
            };

            // 3. Execute handler
            let task_result = handler.handle(&context).await;

            // 4. Handle transaction and job state
            match task_result {
                Ok(_) => {
                    // Handler should have marked the job as 'Completed' inside the transaction.
                    context.uow.commit().await?;
                }
                Err(e) => {
                    eprintln!("Job {} has failed: {}", job_id, e);
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
