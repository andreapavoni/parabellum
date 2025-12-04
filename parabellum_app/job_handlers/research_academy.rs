use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;
use std::sync::Arc;
use tracing::{info, instrument};

use crate::{
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

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::sync::Arc;

    use parabellum_game::test_utils::{
        PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
    };
    use parabellum_types::Result;
    use parabellum_types::{army::UnitName, tribe::Tribe};

    use super::*;
    use crate::{
        config::Config,
        jobs::{Job, JobPayload},
        test_utils::tests::MockUnitOfWork,
        uow::UnitOfWork,
    };

    #[tokio::test]
    async fn test_research_academy_job_handler_success() -> Result<()> {
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

        village.set_academy_research_for_test(&UnitName::Praetorian, false);
        mock_uow.villages().save(&village).await?;

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
        handler.handle(&context, &job).await?;

        let saved_village = context.uow.villages().get_by_id(village_id).await?;
        let unit_idx = saved_village
            .tribe
            .get_unit_idx_by_name(&unit_to_research)
            .unwrap();
        assert!(
            saved_village.academy_research().get(unit_idx),
            "Unit should be marked as researched"
        );
        Ok(())
    }
}
