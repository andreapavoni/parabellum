use crate::{
    cqrs::{
        QueryHandler,
        queries::{GetVillageTrainingQueue, TrainingQueueItem},
    },
    jobs::tasks::TrainUnitsTask,
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

        let mut entries = Vec::with_capacity(jobs.len());
        for job in jobs {
            let Ok(payload) = serde_json::from_value::<TrainUnitsTask>(job.task.data.clone())
            else {
                continue;
            };

            entries.push(TrainingQueueItem {
                job_id: job.id,
                slot_id: payload.slot_id,
                unit: payload.unit,
                quantity: payload.quantity,
                time_per_unit: payload.time_per_unit,
                status: job.status.clone(),
                finishes_at: job.completed_at,
            });
        }

        entries.sort_by_key(|item| item.finishes_at);
        Ok(entries)
    }
}
