use crate::{
    cqrs::{
        QueryHandler,
        queries::{AcademyQueueItem, GetVillageAcademyQueue},
    },
    queries_handlers::queue_converters::academy_queue_item_from_job,
    uow::UnitOfWork,
};
use parabellum_types::{Result, errors::ApplicationError};

pub struct GetVillageAcademyQueueHandler;

impl GetVillageAcademyQueueHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl QueryHandler<GetVillageAcademyQueue> for GetVillageAcademyQueueHandler {
    async fn handle(
        &self,
        query: GetVillageAcademyQueue,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &std::sync::Arc<crate::config::Config>,
    ) -> Result<<GetVillageAcademyQueue as crate::cqrs::Query>::Output, ApplicationError> {
        let job_repo = uow.jobs();
        let jobs = job_repo
            .list_village_academy_queue(query.village_id as i32)
            .await?;

        let mut entries: Vec<AcademyQueueItem> = jobs
            .iter()
            .filter_map(academy_queue_item_from_job)
            .collect();

        entries.sort_by_key(|item| item.finishes_at);
        Ok(entries)
    }
}
