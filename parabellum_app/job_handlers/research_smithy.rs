use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, instrument};

use parabellum_types::errors::ApplicationError;

use crate::{
    jobs::{
        Job,
        handler::{JobHandler, JobHandlerContext},
        tasks::ResearchSmithyTask,
    },
    repository::VillageRepository,
};

pub struct ResearchSmithyJobHandler {
    payload: ResearchSmithyTask,
}

impl ResearchSmithyJobHandler {
    pub fn new(payload: ResearchSmithyTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for ResearchSmithyJobHandler {
    #[instrument(skip_all, fields(
        task_type = "ResearchSmithy",
        unit = ?self.payload.unit,
        village_id = job.village_id
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Executing ResearchSmithy job");

        let village_repo: Arc<dyn VillageRepository + '_> = ctx.uow.villages();
        let village_id = job.village_id as u32;

        let mut village = village_repo.get_by_id(village_id).await?;

        village.upgrade_smithy(self.payload.unit.clone())?;
        village_repo.save(&village).await?;

        info!(unit = ?self.payload.unit, "Smithy upgrade completed.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::sync::Arc;

    use parabellum_types::Result;
    use parabellum_game::test_utils::{
        PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
    };
    use parabellum_types::{army::UnitName, tribe::Tribe};

    use super::*;
    use crate::{
        config::Config,
        jobs::{Job, JobPayload},
        test_utils::tests::MockUnitOfWork,
        uow::UnitOfWork,
    };

    #[tokio::test]
    async fn test_research_smithy_job_handler_success() -> Result<()> {
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

        let unit_to_upgrade = UnitName::Legionnaire;
        let unit_idx = village
            .tribe
            .get_unit_idx_by_name(&unit_to_upgrade)
            .unwrap();
        village.set_smithy_level_for_test(&unit_to_upgrade, 1);
        mock_uow.villages().save(&village).await?;

        let payload = ResearchSmithyTask {
            unit: unit_to_upgrade.clone(),
        };
        let job_payload = JobPayload::new("ResearchSmithy", json!(payload));
        let job = Job::new(player.id, village_id as i32, 0, job_payload);

        let handler = ResearchSmithyJobHandler::new(payload);
        let context = JobHandlerContext {
            uow: mock_uow,
            config,
        };
        handler.handle(&context, &job).await?;

        let saved_village = context.uow.villages().get_by_id(village_id).await?;
        assert_eq!(
            saved_village.smithy()[unit_idx],
            2,
            "Unit smithy level should be incremented to 1"
        );

        Ok(())
    }
}
