use crate::{
    cqrs::{
        QueryHandler,
        queries::{BuildingQueueItem, GetVillageBuildingQueue},
    },
    jobs::tasks::{AddBuildingTask, BuildingUpgradeTask},
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

        let mut entries = Vec::with_capacity(jobs.len());
        for job in jobs {
            let parsed = match job.task.task_type.as_str() {
                "AddBuilding" => serde_json::from_value(job.task.data.clone())
                    .map(|payload: AddBuildingTask| (payload.slot_id, payload.name, 1)),
                "BuildingUpgrade" => serde_json::from_value(job.task.data.clone()).map(
                    |payload: BuildingUpgradeTask| {
                        (payload.slot_id, payload.building_name, payload.level)
                    },
                ),
                _ => continue,
            };

            if let Ok((slot_id, building_name, target_level)) = parsed {
                entries.push(BuildingQueueItem {
                    job_id: job.id,
                    slot_id,
                    building_name,
                    target_level,
                    status: job.status,
                    finishes_at: job.completed_at,
                });
            }
        }

        entries.sort_by_key(|item| item.finishes_at);
        Ok(entries)
    }
}
