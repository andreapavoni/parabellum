use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

use crate::{
    app::job_handlers::attack::AttackJobHandler,
    jobs::{
        handler::{JobHandler, JobHandlerContext},
        JobTask,
    },
    repository::{ArmyRepository, JobRepository, VillageRepository},
};

/// Responsible for polling and execute job.
pub struct JobWorker {
    context: JobHandlerContext,
}

impl JobWorker {
    pub fn new(
        job_repo: Arc<dyn JobRepository>,
        village_repo: Arc<dyn VillageRepository>,
        army_repo: Arc<dyn ArmyRepository>,
    ) -> Self {
        let context = JobHandlerContext {
            job_repo,
            village_repo,
            army_repo,
        };
        Self { context }
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

    async fn process_due_jobs(&self) -> Result<()> {
        let due_jobs = self.context.job_repo.find_and_lock_due_jobs(10).await?;
        if due_jobs.is_empty() {
            return Ok(());
        }
        println!("Found {} jobs to be executed.", due_jobs.len());

        for job in due_jobs {
            let job_id = job.id;

            // --- Dispatch ---
            let handler: Box<dyn JobHandler> = match job.task {
                JobTask::Attack(payload) => Box::new(AttackJobHandler::new(payload)),
                // JobTask::BuildingUpgrade { ... } => Box::new(BuildingUpgradeHandler::new(...)),
                _ => {
                    println!("Task not yet implemented for job {}", job_id);
                    continue;
                }
            };

            // Execute handler and update the job status
            let task_result = handler.handle(&self.context).await;

            match task_result {
                Ok(_) => self.context.job_repo.mark_as_completed(job_id).await?,
                Err(e) => {
                    eprintln!("Job {} has failed: {}", job_id, e);
                    self.context
                        .job_repo
                        .mark_as_failed(job_id, &e.to_string())
                        .await?
                }
            }
        }
        Ok(())
    }
}
