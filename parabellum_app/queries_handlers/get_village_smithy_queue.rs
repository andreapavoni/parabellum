use crate::{
    cqrs::{
        QueryHandler,
        queries::{GetVillageSmithyQueue, SmithyQueueItem},
    },
    jobs::tasks::ResearchSmithyTask,
    uow::UnitOfWork,
};
use parabellum_types::{Result, errors::ApplicationError};

pub struct GetVillageSmithyQueueHandler;

impl GetVillageSmithyQueueHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl QueryHandler<GetVillageSmithyQueue> for GetVillageSmithyQueueHandler {
    async fn handle(
        &self,
        query: GetVillageSmithyQueue,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &std::sync::Arc<crate::config::Config>,
    ) -> Result<<GetVillageSmithyQueue as crate::cqrs::Query>::Output, ApplicationError> {
        let job_repo = uow.jobs();
        let jobs = job_repo
            .list_village_smithy_queue(query.village_id as i32)
            .await?;

        let mut entries = Vec::with_capacity(jobs.len());
        for job in jobs {
            let Ok(payload) = serde_json::from_value::<ResearchSmithyTask>(job.task.data.clone())
            else {
                continue;
            };

            entries.push(SmithyQueueItem {
                job_id: job.id,
                unit: payload.unit,
                status: job.status.clone(),
                finishes_at: job.completed_at,
            });
        }

        entries.sort_by_key(|item| item.finishes_at);
        Ok(entries)
    }
}
