use std::sync::Arc;

use uuid::Uuid;

use parabellum_types::errors::ApplicationError;

use crate::{
    command_handlers::helpers::{
        build_scheduled_building_queue_job, building_queue_plan_from_event,
    },
    cqrs_es::village::VillageEvent,
    jobs::Job,
    repository::JobRepository,
};

pub struct BuildingJobsConsumer<'a> {
    pub job_repo: Arc<dyn JobRepository + 'a>,
    pub player_id: Uuid,
    pub village_id: i32,
    pub existing_building_jobs: Vec<Job>,
    pub duration_secs: i64,
}

impl<'a> BuildingJobsConsumer<'a> {
    pub async fn consume(&self, event: &VillageEvent) -> Result<Job, ApplicationError> {
        let Some(plan) = building_queue_plan_from_event(self.village_id, event) else {
            return Err(ApplicationError::Unknown(
                "failed to build job plan from building queue event".to_string(),
            ));
        };

        let new_job = build_scheduled_building_queue_job(
            self.player_id,
            self.village_id,
            &self.existing_building_jobs,
            self.duration_secs,
            plan,
        )?;
        self.job_repo.add(&new_job).await?;
        Ok(new_job)
    }
}
