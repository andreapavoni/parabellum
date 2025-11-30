use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_types::errors::ApplicationError;

use crate::jobs::{
    Job,
    handler::{JobHandler, JobHandlerContext},
    tasks::ArmyReturnTask,
};

pub struct ArmyReturnJobHandler {
    payload: ArmyReturnTask,
}

impl ArmyReturnJobHandler {
    pub fn new(payload: ArmyReturnTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for ArmyReturnJobHandler {
    #[instrument(skip_all, fields(
        task_type = "ArmyReturn",
        army_id = ?job.task.data.get("army_id"),
        player_id = %job.player_id,
        village_id = job.village_id,
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Executing ArmyReturn job");

        let village_id = job.village_id as u32;
        let army_repo = ctx.uow.armies();
        let village_repo = ctx.uow.villages();

        let mut village = village_repo.get_by_id(village_id).await?;
        let returning_army = army_repo.get_by_id(self.payload.army_id).await?;
        village.merge_army(&returning_army)?;

        army_repo.save(village.army().unwrap()).await?;
        army_repo.remove(returning_army.id).await?;
        village.store_resources(&self.payload.resources);
        village_repo.save(&village).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::sync::Arc;
    use uuid::Uuid;

    use parabellum_game::test_utils::{
        ArmyFactoryOptions, PlayerFactoryOptions, VillageFactoryOptions, army_factory,
        player_factory, village_factory,
    };
    use parabellum_types::Result;
    use parabellum_types::{common::ResourceGroup, tribe::Tribe};

    use super::*;
    use crate::{
        config::Config,
        jobs::{Job, JobPayload},
        test_utils::tests::{MockUnitOfWork, set_village_resources},
        uow::UnitOfWork,
    };

    fn setup_test_job(
        task: ArmyReturnTask,
        player_id: Uuid,
        village_id: i32,
    ) -> Result<(Job, Arc<Config>, Box<dyn UnitOfWork<'static> + 'static>)> {
        let job_payload = JobPayload::new("ArmyReturn", json!(task));
        let job = Job::new(player_id, village_id, 0, job_payload);
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());
        Ok((job, config, mock_uow))
    }

    #[tokio::test]
    async fn test_army_return_merges_armies_and_removes_returning() -> Result<()> {
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });
        set_village_resources(&mut village, ResourceGroup::default());

        // Home army has 10 legionnaires
        let home_army = army_factory(ArmyFactoryOptions {
            village_id: Some(village.id),
            player_id: Some(player.id),
            tribe: Some(player.tribe.clone()),
            units: Some([10, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            ..Default::default()
        });
        village.set_army(Some(&home_army))?;

        // Returning army has 5 legionnaires
        let returning_army = army_factory(ArmyFactoryOptions {
            village_id: Some(village.id), // Original village
            player_id: Some(player.id),
            tribe: Some(player.tribe.clone()),
            units: Some([5, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            ..Default::default()
        });

        let bounty = ResourceGroup::new(100, 100, 100, 100);
        let task = ArmyReturnTask {
            army_id: returning_army.id,
            resources: bounty.clone(),
            destination_village_id: village.id as i32,
            destination_player_id: player.id,
            from_village_id: 2, // Some other village
        };

        let (job, config, uow) = setup_test_job(task.clone(), player.id, village.id as i32)?;

        uow.villages().save(&village).await?;
        uow.armies().save(&home_army).await?;
        uow.armies().save(&returning_army).await?;

        let handler = ArmyReturnJobHandler::new(task);
        let context = JobHandlerContext { uow, config };

        let result = handler.handle(&context, &job).await;
        assert!(result.is_ok(), "Handler failed: {:?}", result.err());

        let final_village = context.uow.villages().get_by_id(village.id).await?;
        let final_home_army = final_village.army().expect("Village should have an army");
        assert_eq!(
            final_home_army.units()[0],
            15,
            "Home army should have 15 units"
        );

        assert_eq!(
            final_village.stored_resources().lumber(),
            bounty.0,
            "Home stocks should be increased with bounty"
        );

        let deleted_army = context.uow.armies().get_by_id(returning_army.id).await;
        assert!(
            deleted_army.is_err(),
            "Returning army should be deleted from db"
        );
        Ok(())
    }
}
