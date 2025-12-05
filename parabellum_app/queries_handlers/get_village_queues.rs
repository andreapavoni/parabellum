use crate::{
    cqrs::{
        QueryHandler,
        queries::{GetVillageQueues, VillageQueues},
    },
    queries_handlers::queue_converters::{
        academy_queue_item_from_job, building_queue_item_from_job, smithy_queue_item_from_job,
        training_queue_item_from_job,
    },
    uow::UnitOfWork,
};
use parabellum_types::{Result, errors::ApplicationError};

pub struct GetVillageQueuesHandler;

impl GetVillageQueuesHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl QueryHandler<GetVillageQueues> for GetVillageQueuesHandler {
    async fn handle(
        &self,
        query: GetVillageQueues,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &std::sync::Arc<crate::config::Config>,
    ) -> Result<VillageQueues, ApplicationError> {
        let job_repo = uow.jobs();
        let jobs = job_repo
            .list_active_jobs_by_village(query.village_id as i32)
            .await?;

        let mut building = Vec::new();
        let mut training = Vec::new();
        let mut academy = Vec::new();
        let mut smithy = Vec::new();

        for job in &jobs {
            if let Some(item) = building_queue_item_from_job(job) {
                building.push(item);
                continue;
            }

            if let Some(item) = training_queue_item_from_job(job) {
                training.push(item);
                continue;
            }

            if let Some(item) = academy_queue_item_from_job(job) {
                academy.push(item);
                continue;
            }

            if let Some(item) = smithy_queue_item_from_job(job) {
                smithy.push(item);
            }
        }

        building.sort_by_key(|item| item.finishes_at);
        training.sort_by_key(|item| item.finishes_at);
        academy.sort_by_key(|item| item.finishes_at);
        smithy.sort_by_key(|item| item.finishes_at);

        Ok(VillageQueues {
            building,
            training,
            academy,
            smithy,
        })
    }
}
