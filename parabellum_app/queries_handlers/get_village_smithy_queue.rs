use crate::{
    cqrs::{
        QueryHandler,
        queries::{GetVillageSmithyQueue, SmithyQueueItem},
    },
    queries_handlers::queue_converters::smithy_queue_item_from_job,
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

        let mut entries: Vec<SmithyQueueItem> =
            jobs.iter().filter_map(smithy_queue_item_from_job).collect();

        entries.sort_by_key(|item| item.finishes_at);
        Ok(entries)
    }
}
