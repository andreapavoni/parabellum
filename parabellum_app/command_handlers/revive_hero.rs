use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

use parabellum_types::errors::{ApplicationError, GameError};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::ReviveHero},
    jobs::{Job, JobPayload, tasks::HeroRevivalTask},
    uow::UnitOfWork,
};

pub struct ReviveHeroCommandHandler;

impl ReviveHeroCommandHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReviveHeroCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler<ReviveHero> for ReviveHeroCommandHandler {
    async fn handle(
        &self,
        command: ReviveHero,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let hero_repo = uow.heroes();
        let village_repo = uow.villages();
        let job_repo = uow.jobs();

        let mut hero = hero_repo.get_by_id(command.hero_id).await?;
        if hero.player_id != command.player_id {
            return Err(ApplicationError::Game(GameError::HeroNotOwned {
                hero_id: hero.id,
                player_id: command.player_id,
            }));
        }

        if hero.is_alive() {
            return Err(ApplicationError::Game(GameError::HeroNotDead));
        }

        let mut village = village_repo.get_by_id(command.village_id).await?;
        if village.player_id != command.player_id {
            return Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: village.id,
                player_id: command.player_id,
            }));
        }

        let cost = if command.reset {
            hero.resurrection_cost(config.speed)
        } else {
            hero.level = 0;
            hero.experience = 0;
            hero.unassigned_points = 5;
            hero.resurrection_cost(config.speed)
        };

        village.deduct_resources(&cost.resources)?;
        village_repo.save(&village).await?;

        let payload = HeroRevivalTask {
            hero_id: hero.id,
            player_id: hero.player_id,
            village_id: command.village_id as i32,
            reset: command.reset,
        };
        let job_payload = JobPayload::new("HeroRevival", serde_json::to_value(&payload)?);

        let job = Job::new(
            command.player_id,
            command.village_id as i32,
            cost.time as i64,
            job_payload,
        );

        job_repo.add(&job).await?;

        info!(
            job_id = %job.id,
            hero_id = %hero.id,
            reset = ?command.reset,
            revival_at = %job.completed_at,
            "Hero revival job scheduled"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use parabellum_game::{models::buildings::Building, test_utils::setup_player_party};
    use parabellum_types::Result;
    use parabellum_types::{buildings::BuildingName, common::ResourceGroup, tribe::Tribe};

    use super::*;
    use crate::test_utils::tests::MockUnitOfWork;

    #[tokio::test]
    async fn cannot_revive_alive_hero() -> Result<()> {
        let uow: Box<dyn UnitOfWork> = Box::new(MockUnitOfWork::new());
        let hero_repo = uow.heroes();
        let village_repo = uow.villages();

        let config = Arc::new(Config::from_env());
        let (player, village, _, hero) = setup_player_party(None, Tribe::Roman, [0; 10], true)?;
        let hero = hero.unwrap();

        hero_repo.save(&hero).await.unwrap();
        village_repo.save(&village).await.unwrap();

        let handler = ReviveHeroCommandHandler::new();
        let cmd = ReviveHero {
            player_id: player.id,
            hero_id: hero.id,
            village_id: village.id,
            reset: false,
        };

        let result = handler.handle(cmd, &uow, &config).await;
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn schedules_job_and_consumes_resources_for_existing() -> Result<()> {
        let uow: Box<dyn UnitOfWork> = Box::new(MockUnitOfWork::new());
        let hero_repo = uow.heroes();
        let village_repo = uow.villages();
        let job_repo = uow.jobs();

        let config = Arc::new(Config::from_env());
        let (player, mut village, _, hero) = setup_player_party(None, Tribe::Roman, [0; 10], true)?;

        let granary = Building::new(BuildingName::Granary, 1).at_level(10, 1)?;
        let warehouse = Building::new(BuildingName::Warehouse, 1).at_level(10, 1)?;
        village.add_building_at_slot(granary, 22)?;
        village.add_building_at_slot(warehouse, 23)?;
        village.store_resources(&ResourceGroup(5000, 5000, 5000, 5000));

        let initial_resources = village.stored_resources();
        let mut hero = hero.unwrap();
        hero.health = 0;
        hero.level = 0;

        hero_repo.save(&hero).await?;
        village_repo.save(&village).await?;

        let handler = ReviveHeroCommandHandler::new();
        let cmd = ReviveHero {
            player_id: player.id,
            hero_id: hero.id,
            village_id: village.id,
            reset: false,
        };
        handler.handle(cmd, &uow, &config).await?;

        let jobs = job_repo.list_by_player_id(player.id).await.unwrap();
        assert_eq!(jobs.len(), 1);
        let job = &jobs[0];
        assert_eq!(job.task.task_type, "HeroRevival");

        let village = village_repo.get_by_id(village.id).await.unwrap();
        assert!(village.stored_resources().lumber() < initial_resources.lumber());

        Ok(())
    }
}
