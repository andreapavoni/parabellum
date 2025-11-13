mod test_utils;

#[cfg(test)]
pub mod tests {
    use crate::test_utils::tests::setup_player_party;

    use super::test_utils::tests::TestUnitOfWorkProvider;
    use parabellum_app::{
        command_handlers::ReinforceVillageCommandHandler,
        config::Config,
        cqrs::{CommandHandler, commands::ReinforceVillage},
        job_registry::AppJobRegistry,
        jobs::{JobStatus, tasks::ReinforcementTask, worker::JobWorker},
        uow::UnitOfWorkProvider,
    };
    use parabellum_core::Result;
    use parabellum_db::establish_test_connection_pool;
    use parabellum_game::models::buildings::Building;
    use parabellum_game::models::hero::Hero;
    use parabellum_game::test_utils::{
        ArmyFactoryOptions, PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions,
        army_factory, player_factory, valley_factory, village_factory,
    };
    use parabellum_types::{buildings::BuildingName, common::Player, tribe::Tribe};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_transfer_hero_other_village() -> Result<()> {
        let pool = establish_test_connection_pool().await.unwrap();
        let master_tx = pool.begin().await.unwrap();
        let master_tx_arc = Arc::new(Mutex::new(master_tx));
        let uow_provider: Arc<dyn UnitOfWorkProvider> =
            Arc::new(TestUnitOfWorkProvider::new(master_tx_arc.clone()));
        let app_registry = Arc::new(AppJobRegistry::new());
        let config = Arc::new(Config::from_env());

        let player = PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        };
        let position1 = Some(parabellum_types::map::Position { x: 10, y: 10 });
        let position2 = Some(parabellum_types::map::Position { x: 12, y: 12 });
        // Begin a unit of work to set up initial state
        let uow_setup = uow_provider.begin().await?;
        let (player, village1, army1, village2, hero) = {
            let player_repo = uow_setup.players();
            let village_repo = uow_setup.villages();
            let army_repo = uow_setup.armies();
            let hero_repo = uow_setup.heroes();

            let player: Player = player_factory(player);
            player_repo.save(&player).await?;
            // Two villages for the same player
            let valley1 = valley_factory(ValleyFactoryOptions {
                position: position1,
                ..Default::default()
            });
            let valley2 = valley_factory(ValleyFactoryOptions {
                position: position2,
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
            // Add HeroMansion to village2
            let mansion =
                Building::new(BuildingName::HeroMansion, config.speed).at_level(1, config.speed)?;

            village2.add_building_at_slot(mansion, 20)?;
            village_repo.save(&village1).await?;
            village_repo.save(&village2).await?;
            // Create an army with some troops and a hero in village1
            let hero = Hero::new(None, village1.id, player.id);
            hero_repo.save(&hero).await?;

            let army1 = army_factory(ArmyFactoryOptions {
                player_id: Some(player.id),
                village_id: Some(village1.id),
                tribe: Some(player.tribe.clone()),
                units: None,
                hero: Some(hero.clone()),
                ..Default::default()
            });
            army_repo.save(&army1).await?;
            (player, village1, army1, village2, hero)
        };
        uow_setup.commit().await?;

        // Issue the ReinforceVillage command (with hero) to send from village1 to village2
        let reinforce_cmd = ReinforceVillage {
            player_id: player.id,
            village_id: village1.id,
            army_id: army1.id,
            units: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: (village2.id),
            hero_id: Some(hero.id),
        };
        {
            let uow_cmd = uow_provider.begin().await?;
            ReinforceVillageCommandHandler::new()
                .handle(reinforce_cmd, &uow_cmd, &config)
                .await?;
            uow_cmd.commit().await?;
        }

        // Verify the job is queued correctly
        let (reinforce_job, _deployed_army_id) = {
            let uow_check = uow_provider.begin().await?;
            let jobs = uow_check.jobs().list_by_player_id(player.id).await?;
            assert_eq!(jobs.len(), 1, "There should be 1 pending job");
            let job = jobs[0].clone();
            assert_eq!(job.task.task_type, "Reinforcement");
            let payload: ReinforcementTask = serde_json::from_value(job.task.data.clone())?;
            assert_ne!(payload.army_id, army1.id, "Deployed army ID should be new");
            assert_eq!(
                payload.village_id, village2.id as i32,
                "Target village ID should match"
            );
            // Source village army should be removed, and all troops (and hero) departed
            let src_village = uow_check.villages().get_by_id(village1.id).await?;
            assert!(
                src_village.army().is_none(),
                "Source village army should be None after departure"
            );
            assert!(
                uow_check.armies().get_by_id(army1.id).await.is_err(),
                "Original army should be deleted"
            );
            (job.clone(), payload.army_id)
        };
        // Process the reinforcement job (simulate troops arriving)
        let worker = Arc::new(JobWorker::new(
            uow_provider.clone(),
            app_registry.clone(),
            config.clone(),
        ));
        worker.process_jobs(&vec![reinforce_job.clone()]).await?;

        // Verify post-arrival state in the database
        {
            let uow_final = uow_provider.begin().await?;
            let completed_job = uow_final.jobs().get_by_id(reinforce_job.id).await?;
            assert_eq!(
                completed_job.status,
                JobStatus::Completed,
                "Reinforcement job should be completed"
            );
            let remaining_jobs = uow_final.jobs().list_by_player_id(player.id).await?;
            assert!(remaining_jobs.is_empty(), "No return job should remain");
            let target_village = uow_final.villages().get_by_id(village2.id).await?; // village2
            assert_eq!(
                target_village.reinforcements().len(),
                0,
                "Expected reinforcements 0 in target village"
            );

            assert!(
                target_village.army().is_some(),
                "Target village should have an army at home"
            );

            let home_army = target_village.army().unwrap();
            assert!(
                home_army.hero().is_some(),
                "Target village should have 0 units at home"
            );

            let hero = home_army.hero().unwrap();
            assert_eq!(
                hero.village_id, target_village.id,
                "Hero should belong to target village"
            );

            uow_final.rollback().await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_full_reinforce_flow_hero_other_player() -> Result<()> {
        let pool = establish_test_connection_pool().await.unwrap();
        let master_tx = pool.begin().await.unwrap();
        let master_tx_arc = Arc::new(Mutex::new(master_tx));
        let uow_provider: Arc<dyn UnitOfWorkProvider> =
            Arc::new(TestUnitOfWorkProvider::new(master_tx_arc.clone()));
        let app_registry = Arc::new(AppJobRegistry::new());
        let config = Arc::new(Config::from_env());

        let (reinforcer_player, reinforcer_village, reinforcer_army, hero) = {
            // Use helper to create player + village + army with hero for reinforcer
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Teuton,
                [20, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                true,
            )
            .await?
        };
        let (_target_player, target_village, _target_army, _target_hero) = {
            // Create target player’s village (no troops)
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Attach a hero to the reinforcer's army
        {
            let uow_attach = uow_provider.begin().await?;
            let army_repo = uow_attach.armies();

            let mut army = army_repo.get_by_id(reinforcer_army.id).await?;
            army.set_hero(hero.clone());
            army_repo.save(&army).await?;
            uow_attach.commit().await?;
        }

        // Send reinforcement with hero to the other player's village
        let reinforce_cmd = ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: reinforcer_army.id,
            units: [20, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: target_village.id,
            hero_id: hero.clone().map(|h| h.id), // (use the hero attached above)
        };
        {
            let uow_cmd = uow_provider.begin().await?;
            ReinforceVillageCommandHandler::new()
                .handle(reinforce_cmd, &uow_cmd, &config)
                .await?;
            uow_cmd.commit().await?;
        }

        // Check the queued job
        let (reinforce_job, deployed_army_id) = {
            let uow_check = uow_provider.begin().await?;
            let jobs = uow_check
                .jobs()
                .list_by_player_id(reinforcer_player.id)
                .await?;
            assert_eq!(jobs.len(), 1);
            let job = jobs[0].clone();
            assert_eq!(job.task.task_type, "Reinforcement");
            let payload: ReinforcementTask = serde_json::from_value(job.task.data.clone())?;
            assert_ne!(payload.army_id, reinforcer_army.id);
            assert_eq!(payload.village_id, target_village.id as i32);
            // Reinforcer's village should have no army left
            let src_village = uow_check
                .villages()
                .get_by_id(reinforcer_village.id)
                .await?;
            assert!(src_village.army().is_none());
            assert!(
                uow_check
                    .armies()
                    .get_by_id(reinforcer_army.id)
                    .await
                    .is_err()
            );
            (job.clone(), payload.army_id)
        };
        // Process the reinforcement job
        let worker = Arc::new(JobWorker::new(
            uow_provider.clone(),
            app_registry.clone(),
            config.clone(),
        ));
        worker.process_jobs(&vec![reinforce_job.clone()]).await?;

        // Verify post-arrival state
        {
            let uow_final = uow_provider.begin().await?;
            let completed_job = uow_final.jobs().get_by_id(reinforce_job.id).await?;
            assert_eq!(completed_job.status, JobStatus::Completed);
            // After arrival, the reinforcements should remain as an allied army in the target village
            let target_village_state = uow_final.villages().get_by_id(target_village.id).await?;
            assert_eq!(
                target_village_state.reinforcements().len(),
                1,
                "Target village should have 1 reinforcement army present"
            );
            assert_eq!(
                target_village_state.reinforcements()[0].id,
                deployed_army_id,
                "Reinforcement army ID should match deployed army"
            );
            // The hero remains with the original player's army (ownership unchanged)
            let deployed_army = uow_final.armies().get_by_id(deployed_army_id).await?;
            assert_eq!(
                deployed_army.player_id, reinforcer_player.id,
                "Reinforcement army remains under original player's control"
            );
            assert!(
                target_village_state.army().is_some(),
                "Target village keeps its own army separate"
            );

            // ------- new

            assert_eq!(
                deployed_army.village_id, reinforcer_village.id,
                "Reinforcement should belong to reinforcer"
            );

            assert_eq!(
                deployed_army.current_map_field_id,
                Some(target_village.id),
                "Reinforcement should stay in target village"
            );

            assert!(
                deployed_army.hero().is_some(),
                "Hero should stay with reinforcements in target village"
            );
            uow_final.rollback().await?;
        }
        Ok(())
    }
}
