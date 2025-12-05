use crate::{
    cqrs::{
        QueryHandler,
        queries::{BuildingQueueItem, GetVillageBuildingQueue},
    },
    queries_handlers::queue_converters::building_queue_item_from_job,
    uow::UnitOfWork,
};
use parabellum_types::{Result, errors::ApplicationError};

pub struct GetVillageBuildingQueueHandler {}

impl GetVillageBuildingQueueHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl QueryHandler<GetVillageBuildingQueue> for GetVillageBuildingQueueHandler {
    async fn handle(
        &self,
        query: GetVillageBuildingQueue,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &std::sync::Arc<crate::config::Config>,
    ) -> Result<<GetVillageBuildingQueue as crate::cqrs::Query>::Output, ApplicationError> {
        let job_repo = uow.jobs();
        let jobs = job_repo
            .list_village_building_queue(query.village_id as i32)
            .await?;

        let mut entries: Vec<BuildingQueueItem> = jobs
            .iter()
            .filter_map(building_queue_item_from_job)
            .collect();

        entries.sort_by_key(|item| item.finishes_at);
        Ok(entries)
    }
}
