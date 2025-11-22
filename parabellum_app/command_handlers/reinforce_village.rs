use std::sync::Arc;
use tracing::info;

use parabellum_types::{Result, errors::ApplicationError};

use crate::{
    command_handlers::helpers::deploy_army_from_village,
    config::Config,
    cqrs::{CommandHandler, commands::ReinforceVillage},
    jobs::{Job, JobPayload, tasks::ReinforcementTask},
    uow::UnitOfWork,
};

pub struct ReinforceVillageCommandHandler {}

impl Default for ReinforceVillageCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ReinforceVillageCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<ReinforceVillage> for ReinforceVillageCommandHandler {
    async fn handle(
        &self,
        command: ReinforceVillage,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let job_repo = uow.jobs();
        let village_repo = uow.villages();
        let reinforcer_village = village_repo.get_by_id(command.village_id).await?;
        let target_village = village_repo.get_by_id(command.target_village_id).await?;
        let (reinforcer_village, deployed_army) = deploy_army_from_village(
            uow,
            reinforcer_village,
            command.army_id,
            command.units,
            command.hero_id,
        )
        .await?;

        let travel_time_secs = reinforcer_village.position.calculate_travel_time_secs(
            target_village.position,
            deployed_army.speed(),
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        let reinforce_payload = ReinforcementTask {
            army_id: deployed_army.id,
            village_id: command.target_village_id as i32,
            player_id: command.player_id,
        };

        let job_payload =
            JobPayload::new("Reinforcement", serde_json::to_value(&reinforce_payload)?);
        let new_job = Job::new(
            command.player_id,
            command.village_id as i32,
            travel_time_secs,
            job_payload,
        );
        job_repo.add(&new_job).await?;
        info!(
            reinforce_job_id = %new_job.id,
            arrival_at = %new_job.completed_at,
            "Reinforcement job planned."
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::tests::MockUnitOfWork;
    use parabellum_game::models::{buildings::Building, hero::Hero};
    use parabellum_game::test_utils::{
        ArmyFactoryOptions, PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions,
        army_factory, player_factory, valley_factory, village_factory,
    };
    use parabellum_types::{buildings::BuildingName, map::Position, tribe::Tribe};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_reinforce_village_handler_with_hero_same_player() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let village_repo = mock_uow.villages();
        let army_repo = mock_uow.armies();
        let job_repo = mock_uow.jobs();
        let hero_repo = mock_uow.heroes();
        let config = Arc::new(Config::from_env());

        // One player with two villages
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        // Create two distinct valley positions for the villages
        let valley1 = valley_factory(ValleyFactoryOptions {
            position: Some(Position { x: 0, y: 0 }),
            ..Default::default()
        });
        let valley2 = valley_factory(ValleyFactoryOptions {
            position: Some(Position { x: 5, y: 5 }),
            ..Default::default()
        });
        let village1 = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            valley: Some(valley1),
            ..Default::default()
        });
        let mut village2 = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            valley: Some(valley2),
            ..Default::default()
        });
        // Give village2 a HeroMansion to allow hero relocation
        let hero_mansion = Building::new(BuildingName::HeroMansion, config.speed)
            .at_level(1, config.speed)
            .unwrap();
        village2.add_building_at_slot(hero_mansion, 20).unwrap(); // assume slot 20 is free for HeroMansion

        // Create a hero and an army in village1 (source)
        let hero = Hero::new(None, village1.id, player.id, player.tribe, None);
        let source_army = army_factory(ArmyFactoryOptions {
            player_id: Some(player.id),
            village_id: Some(village1.id),
            tribe: Some(Tribe::Roman),
            units: Some([10, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            hero: Some(hero.clone()),
            ..Default::default()
        });

        // (Optionally, target village could have its own army; here we skip creating one since it's not needed for sending logic)
        village_repo.save(&village1).await.unwrap();
        village_repo.save(&village2).await.unwrap();
        hero_repo.save(&hero).await.unwrap();
        army_repo.save(&source_army).await.unwrap();

        // Execute ReinforceVillage command with hero_id (reinforcing player's own second village)
        let handler = ReinforceVillageCommandHandler::new();
        let command = ReinforceVillage {
            player_id: player.id,
            village_id: village1.id,
            army_id: source_army.id,
            units: [10, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: village2.id,
            hero_id: Some(hero.id),
        };
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().unwrap().to_string()
        );

        // Verify a reinforcement job is created
        let jobs = job_repo.list_by_player_id(player.id).await.unwrap();
        assert_eq!(jobs.len(), 1, "One job should be created");
        let job = &jobs[0];
        assert_eq!(job.task.task_type, "Reinforcement");
        let reinforce_task: ReinforcementTask =
            serde_json::from_value(job.task.data.clone()).unwrap();
        let deployed_army_id = reinforce_task.army_id;
        assert_ne!(
            deployed_army_id, source_army.id,
            "Deployed army should have a new ID"
        );
        assert_eq!(
            reinforce_task.village_id, village2.id as i32,
            "Target village ID in task should match"
        );

        // The source village's army should be gone (all troops and hero left)
        let home_army_res = army_repo.get_by_id(source_army.id).await;
        assert!(
            home_army_res.is_err(),
            "Source village's army should be removed after sending all units and hero"
        );
        let updated_village1 = village_repo.get_by_id(village1.id).await.unwrap();
        assert!(
            updated_village1.army().is_none(),
            "Source village should have no army after reinforcement"
        );

        // The deployed reinforcement army should include the hero at dispatch time
        let deployed_army = army_repo.get_by_id(deployed_army_id).await.unwrap();
        assert!(
            deployed_army.hero().is_some(),
            "Hero should accompany the reinforcing army"
        );
        assert_eq!(
            deployed_army.hero().unwrap().id,
            source_army.hero().unwrap().id,
            "Hero ID should match the one sent with the reinforcements"
        );
        assert_eq!(
            deployed_army.player_id, player.id,
            "Deployed army should belong to the same player (hero owner)"
        );
    }

    #[tokio::test]
    async fn test_reinforce_village_handler_with_hero_other_player() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let village_repo = mock_uow.villages();
        let army_repo = mock_uow.armies();
        let job_repo = mock_uow.jobs();
        let hero_repo = mock_uow.heroes();
        let config = Arc::new(Config::from_env());

        // Two different players and their villages
        let reinforcer_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Teuton),
            ..Default::default()
        });
        let target_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        let reinforcer_village = village_factory(VillageFactoryOptions {
            player: Some(reinforcer_player.clone()),
            valley: Some(valley_factory(Default::default())),
            ..Default::default()
        });
        let target_village = village_factory(VillageFactoryOptions {
            player: Some(target_player.clone()),
            valley: Some(valley_factory(ValleyFactoryOptions {
                position: Some(Position { x: 8, y: 8 }),
                ..Default::default()
            })),
            ..Default::default()
        });

        // Create a hero and army in the reinforcer's village
        let hero = Hero::new(
            None,
            reinforcer_village.id,
            reinforcer_player.id,
            reinforcer_player.tribe,
            None,
        );
        let reinforcer_army = army_factory(ArmyFactoryOptions {
            player_id: Some(reinforcer_player.id),
            village_id: Some(reinforcer_village.id),
            tribe: Some(Tribe::Teuton),
            units: Some([12, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            hero: Some(hero.clone()),
            ..Default::default()
        });

        village_repo.save(&reinforcer_village).await.unwrap();
        village_repo.save(&target_village).await.unwrap();
        hero_repo.save(&hero).await.unwrap();
        army_repo.save(&reinforcer_army).await.unwrap();

        // Execute ReinforceVillage command to another player's village (hero_id provided)
        let handler = ReinforceVillageCommandHandler::new();
        let command = ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: reinforcer_army.id,
            units: [12, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: target_village.id,
            hero_id: Some(reinforcer_army.hero().unwrap().id),
        };
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().unwrap().to_string()
        );

        // Verify job creation
        let jobs = job_repo
            .list_by_player_id(reinforcer_player.id)
            .await
            .unwrap();
        assert_eq!(jobs.len(), 1, "One job should be created");
        let job = &jobs[0];
        assert_eq!(job.task.task_type, "Reinforcement");
        let reinforce_task: ReinforcementTask =
            serde_json::from_value(job.task.data.clone()).unwrap();
        let deployed_army_id = reinforce_task.army_id;
        assert_ne!(deployed_army_id, reinforcer_army.id);
        assert_eq!(reinforce_task.village_id, target_village.id as i32);

        // Source village army removed
        assert!(army_repo.get_by_id(reinforcer_army.id).await.is_err());
        let updated_reinforcer_village =
            village_repo.get_by_id(reinforcer_village.id).await.unwrap();
        assert!(updated_reinforcer_village.army().is_none());

        // Deployed army has hero at send time
        let deployed_army = army_repo.get_by_id(deployed_army_id).await.unwrap();
        assert!(
            deployed_army.hero().is_some(),
            "Hero should travel with the reinforcing army"
        );
        assert_eq!(
            deployed_army.hero().unwrap().id,
            reinforcer_army.hero().unwrap().id
        );
        // The hero and army remain under the reinforcer player's ownership
        assert_eq!(
            deployed_army.player_id, reinforcer_player.id,
            "Reinforcement army should remain under original player's control"
        );
    }
}
