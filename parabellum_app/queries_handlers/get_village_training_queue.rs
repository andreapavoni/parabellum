use crate::{
    cqrs::{
        QueryHandler,
        queries::{GetVillageTrainingQueue, TrainingQueueItem},
    },
    queries_handlers::queue_converters::training_queue_item_from_job,
    uow::UnitOfWork,
};
use parabellum_types::{Result, errors::ApplicationError};

pub struct GetVillageTrainingQueueHandler;

impl GetVillageTrainingQueueHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl QueryHandler<GetVillageTrainingQueue> for GetVillageTrainingQueueHandler {
    async fn handle(
        &self,
        query: GetVillageTrainingQueue,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &std::sync::Arc<crate::config::Config>,
    ) -> Result<<GetVillageTrainingQueue as crate::cqrs::Query>::Output, ApplicationError> {
        let job_repo = uow.jobs();
        let jobs = job_repo
            .list_village_training_queue(query.village_id as i32)
            .await?;

        let mut entries: Vec<TrainingQueueItem> = jobs
            .iter()
            .filter_map(training_queue_item_from_job)
            .collect();

        entries.sort_by_key(|item| item.finishes_at);
        Ok(entries)
    }
}
