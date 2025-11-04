use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, instrument};

use crate::{
    Result,
    error::ApplicationError,
    jobs::{
        Job,
        handler::{JobHandler, JobHandlerContext},
        tasks::ResearchAcademyTask,
    },
    repository::VillageRepository,
};

pub struct ResearchAcademyJobHandler {
    payload: ResearchAcademyTask,
}

impl ResearchAcademyJobHandler {
    pub fn new(payload: ResearchAcademyTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for ResearchAcademyJobHandler {
    #[instrument(skip_all, fields(
        task_type = "ResearchAcademy",
        unit = ?self.payload.unit,
        village_id = job.village_id
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Executing ResearchAcademy job");

        let village_repo: Arc<dyn VillageRepository + '_> = ctx.uow.villages();
        let village_id = job.village_id as u32;

        let mut village = village_repo.get_by_id(village_id).await?;

        village.research_academy(self.payload.unit.clone())?;
        village_repo.save(&village).await?;

        info!(unit = ?self.payload.unit, "Unit research completed.");
        Ok(())
    }
}
// In src/app/job_handlers/research_academy.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::test_utils::tests::MockUnitOfWork,
        config::Config,
        game::{
            models::{Tribe, army::UnitName},
            test_utils::{
                PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
            },
        },
        jobs::{Job, JobPayload},
        repository::uow::UnitOfWork,
    };
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_research_academy_job_handler_success() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let config = Arc::new(Config::from_env());

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });
        let village_id = village.id;

        village.academy_research[1] = false; // Praetorian
        mock_uow.villages().create(&village).await.unwrap();

        let unit_to_research = UnitName::Praetorian;
        let payload = ResearchAcademyTask {
            unit: unit_to_research.clone(),
        };
        let job_payload = JobPayload::new("ResearchAcademy", json!(payload));
        let job = Job::new(player.id, village_id as i32, 0, job_payload);

        let handler = ResearchAcademyJobHandler::new(payload);
        let context = JobHandlerContext {
            uow: mock_uow,
            config,
        };

        let result = handler.handle(&context, &job).await;
        assert!(
            result.is_ok(),
            "Job handler should succeed: {:?}",
            result.err()
        );

        let saved_village = context.uow.villages().get_by_id(village_id).await.unwrap();
        let unit_idx = saved_village
            .tribe
            .get_unit_idx_by_name(&unit_to_research)
            .unwrap();
        assert!(
            saved_village.academy_research[unit_idx],
            "Unit should be marked as researched"
        );
    }
}
