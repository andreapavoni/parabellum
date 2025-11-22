use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_types::errors::{ApplicationError, GameError};

use crate::jobs::{
    Job,
    handler::{JobHandler, JobHandlerContext},
    tasks::HeroRevivalTask,
};

pub struct HeroRevivalJobHandler {
    pub payload: HeroRevivalTask,
}

impl HeroRevivalJobHandler {
    pub fn new(payload: HeroRevivalTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for HeroRevivalJobHandler {
    #[instrument(skip_all, fields(
        task_type = "HeroRevival",
        hero_id = %self.payload.hero_id,
        player_id = %self.payload.player_id,
        village_id = %self.payload.village_id,
        reset = ?self.payload.reset,
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        _job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        let hero_repo = ctx.uow.heroes();
        let village_repo = ctx.uow.villages();

        let mut hero = hero_repo.get_by_id(self.payload.hero_id).await?;
        let village = village_repo
            .get_by_id(self.payload.village_id as u32)
            .await?;

        if hero.player_id != self.payload.player_id {
            return Err(ApplicationError::Game(GameError::HeroNotOwned {
                hero_id: hero.id,
                player_id: self.payload.player_id,
            }));
        }

        if village.player_id != self.payload.player_id {
            return Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: village.id,
                player_id: self.payload.player_id,
            }));
        }

        hero.resurrect(village.id, self.payload.reset);

        hero_repo.save(&hero).await?;
        village_repo.save(&village).await?;

        info!(
            hero_id = %hero.id,
            reset = ?self.payload.reset,
            "Hero revived successfully"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parabellum_types::Result;
    use parabellum_game::test_utils::setup_player_party;
    use parabellum_types::tribe::Tribe;
    use serde_json::json;

    use super::*;
    use crate::{
        config::Config, jobs::JobPayload, test_utils::tests::MockUnitOfWork, uow::UnitOfWork,
    };

    #[tokio::test]
    async fn hero_revival_keeping_level_and_stats() -> Result<()> {
        let uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());
        let hero_repo = uow.heroes();
        let village_repo = uow.villages();

        let config = Arc::new(Config::from_env());
        let (player, village, _, some_hero) =
            setup_player_party(None, Tribe::Roman, [0; 10], true)?;

        let mut hero = some_hero.unwrap();
        hero.level = 10;
        hero.experience = 10_000;
        hero.strength_points = 20;
        hero.health = 0;

        hero_repo.save(&hero).await?;
        village_repo.save(&village).await?;
        assert!(!hero.is_alive());

        let payload = HeroRevivalTask {
            hero_id: hero.id,
            player_id: player.id,
            village_id: village.id as i32,
            reset: false,
        };
        let job_payload = JobPayload::new("HeroRevival", json!(payload.clone()));
        let job = Job::new(player.id, village.id as i32, 0, job_payload);

        let handler = HeroRevivalJobHandler::new(serde_json::from_value(job.task.data.clone())?);
        let context = JobHandlerContext { uow, config };
        handler.handle(&context, &job).await?;

        let revived = hero_repo.get_by_id(hero.id).await?;

        assert!(revived.is_alive());
        assert_eq!(revived.level, 10);
        assert_eq!(revived.experience, 10_000);
        assert_eq!(revived.strength_points, 20);
        assert_eq!(revived.health, 100);
        assert_eq!(revived.village_id, village.id);

        Ok(())
    }

    #[tokio::test]
    async fn hero_revival_new_resets_level_and_points() -> Result<()> {
        let uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());
        let hero_repo = uow.heroes();
        let village_repo = uow.villages();

        let config = Arc::new(Config::from_env());
        let (player, village, _, some_hero) =
            setup_player_party(None, Tribe::Roman, [0; 10], true)?;

        let mut hero = some_hero.unwrap();
        hero.level = 15;
        hero.experience = 50_000;
        hero.strength_points = 30;
        hero.off_bonus_points = 20;
        hero.regeneration_points = 5;
        hero.resources_points = 10;
        hero.health = 0;
        hero_repo.save(&hero).await?;
        village_repo.save(&village).await?;

        let payload = HeroRevivalTask {
            hero_id: hero.id,
            player_id: player.id,
            village_id: village.id as i32,
            reset: true,
        };
        let job_payload = JobPayload::new("HeroRevival", json!(payload.clone()));
        let job = Job::new(player.id, village.id as i32, 0, job_payload);

        let handler = HeroRevivalJobHandler::new(serde_json::from_value(job.task.data.clone())?);
        let context = JobHandlerContext { uow, config };
        handler.handle(&context, &job).await?;

        let revived = hero_repo.get_by_id(hero.id).await?;

        assert_eq!(revived.level, 0);
        assert_eq!(revived.experience, 0);
        assert_eq!(revived.health, 100);
        assert_eq!(revived.strength_points, 0);
        assert_eq!(revived.off_bonus_points, 0);
        assert_eq!(revived.regeneration_points, 0);
        assert_eq!(revived.resources_points, 0);
        assert_eq!(revived.unassigned_points, 5);

        Ok(())
    }
}
