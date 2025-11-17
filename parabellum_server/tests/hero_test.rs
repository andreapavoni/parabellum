mod test_utils;

#[cfg(test)]
pub mod tests {
    use crate::test_utils::tests::{setup_app, setup_player_party};
    use parabellum_app::{
        command_handlers::{ReinforceVillageCommandHandler, ReviveHeroCommandHandler},
        cqrs::commands::{ReinforceVillage, ReviveHero},
        jobs::{JobStatus, tasks::ReinforcementTask},
    };
    use parabellum_core::Result;

    use parabellum_game::models::buildings::Building;
    use parabellum_game::test_utils::{VillageFactoryOptions, village_factory};

    use parabellum_types::{buildings::BuildingName, common::ResourceGroup, tribe::Tribe};

    #[tokio::test]
    async fn test_transfer_hero_other_village() -> Result<()> {
        let (app, worker, uow_provider, config) = setup_app().await?;

        let (player, village1, army1, some_hero) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], true).await?;
        let hero = some_hero.unwrap();

        let village2 = {
            let uow_setup = uow_provider.begin().await?;
            let village_repo = uow_setup.villages();

            let mut village2 = village_factory(VillageFactoryOptions {
                player: Some(player.clone()),
                ..Default::default()
            });

            // Add HeroMansion to village2
            let mansion =
                Building::new(BuildingName::HeroMansion, config.speed).at_level(1, config.speed)?;
            village2.add_building_at_slot(mansion, 20)?;
            village_repo.save(&village2).await?;

            uow_setup.commit().await?;
            village2
        };

        let cmd = ReinforceVillage {
            player_id: player.id,
            village_id: village1.id,
            army_id: army1.id,
            units: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: (village2.id),
            hero_id: Some(hero.id),
        };
        let handler = ReinforceVillageCommandHandler::new();
        app.execute(cmd, handler).await?;

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
    async fn test_hero_reinforce_other_player() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app().await?;

        let (reinforcer_player, reinforcer_village, reinforcer_army, hero) = setup_player_party(
            uow_provider.clone(),
            None,
            Tribe::Teuton,
            [20, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            true,
        )
        .await?;
        let (_target_player, target_village, _target_army, _target_hero) = setup_player_party(
            uow_provider.clone(),
            None,
            Tribe::Gaul,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            false,
        )
        .await?;

        let reinforce_cmd = ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: reinforcer_army.id,
            units: [20, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: target_village.id,
            hero_id: hero.clone().map(|h| h.id),
        };
        let handler = ReinforceVillageCommandHandler::new();
        app.execute(reinforce_cmd, handler).await?;

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

    #[tokio::test]
    async fn test_resurrect_existing_hero() -> Result<()> {
        let (app, worker, uow_provider, config) = setup_app().await?;

        let (player, mut village, _, some_hero) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], true).await?;
        let mut hero = some_hero.unwrap();

        let (player_id, village_id, hero_id) = {
            let uow = uow_provider.begin().await?;
            let village_repo = uow.villages();
            let hero_repo = uow.heroes();

            let granary = Building::new(BuildingName::Granary, config.speed).at_level(20, 1)?;
            let warehouse = Building::new(BuildingName::Warehouse, 1).at_level(20, 1)?;

            village.add_building_at_slot(granary, 21)?;
            village.add_building_at_slot(warehouse, 20)?;
            village.store_resources(&ResourceGroup(100_000, 100_000, 100_000, 100_000));
            village_repo.save(&village).await?;

            hero.level = 8;
            hero.experience = 10_000;
            hero.strength_points = 12;
            hero.off_bonus_points = 8;
            hero.def_bonus_points = 4;
            hero.regeneration_points = 3;
            hero.resources_points = 5;
            hero.health = 100;
            hero_repo.save(&hero).await?;

            uow.commit().await?;
            (player.id, village.id, hero.id)
        };

        {
            let uow = uow_provider.begin().await?;
            let hero_repo = uow.heroes();

            let mut hero = hero_repo.get_by_id(hero_id).await?;
            hero.apply_battle_damage(0.95);
            assert!(!hero.is_alive());

            hero_repo.save(&hero).await?;
            uow.commit().await?;
        }

        let handler = ReviveHeroCommandHandler::new();
        let cmd = ReviveHero {
            player_id,
            hero_id,
            village_id,
            reset: false,
        };
        app.execute(cmd, handler).await?;

        let revival_job_id = {
            let uow = uow_provider.begin().await?;
            let jobs = uow.jobs().list_by_player_id(player_id).await?;
            assert_eq!(jobs.len(), 1);
            let job = jobs[0].clone();
            assert_eq!(job.task.task_type, "HeroRevival");

            let hero = uow.heroes().get_by_id(hero_id).await?;
            assert_eq!(hero.health, 0);

            let job_id = job.id;
            uow.rollback().await?;
            job_id
        };

        {
            let uow = uow_provider.begin().await?;
            let job = uow.jobs().get_by_id(revival_job_id).await?;
            uow.rollback().await?;
            worker.process_jobs(&vec![job.clone()]).await?;
        }

        {
            let uow = uow_provider.begin().await?;
            let hero_repo = uow.heroes();
            let job_repo = uow.jobs();

            let hero = hero_repo.get_by_id(hero_id).await?;
            assert!(hero.is_alive());
            assert_eq!(hero.level, 8);
            assert_eq!(hero.experience, 10_000);
            assert_eq!(hero.strength_points, 12);
            assert_eq!(hero.off_bonus_points, 8);
            assert_eq!(hero.def_bonus_points, 4);
            assert_eq!(hero.regeneration_points, 3);
            assert_eq!(hero.resources_points, 5);
            assert_eq!(hero.village_id, village_id);

            let job = job_repo.get_by_id(revival_job_id).await?;
            assert_eq!(job.status, JobStatus::Completed);

            uow.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_resurrect_new_hero() -> Result<()> {
        let (app, worker, uow_provider, config) = setup_app().await?;

        let uow = uow_provider.begin().await?;
        let village_repo = uow.villages();
        let hero_repo = uow.heroes();

        let (player_id, village_id, hero_id) = {
            let (player, mut village, _, some_hero) =
                setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], true).await?;
            let mut hero = some_hero.unwrap();

            let granary =
                Building::new(BuildingName::Granary, config.speed).at_level(20, config.speed)?;
            let warehouse =
                Building::new(BuildingName::Warehouse, config.speed).at_level(20, config.speed)?;
            village.add_building_at_slot(granary, 21)?;
            village.add_building_at_slot(warehouse, 20)?;

            village.store_resources(&ResourceGroup(100_000, 100_000, 100_000, 100_000));
            village_repo.save(&village).await?;

            hero.level = 5;
            hero.strength_points = 25;
            hero.off_bonus_points = 0;
            hero.def_bonus_points = 0;
            hero.regeneration_points = 0;
            hero.resources_points = 0;
            hero.health = 0;
            hero_repo.save(&hero).await?;

            uow.commit().await?;
            (player.id, village.id, hero.id)
        };

        let handler = ReviveHeroCommandHandler::new();
        let cmd = ReviveHero {
            player_id,
            hero_id,
            village_id,
            reset: true,
        };
        app.execute(cmd, handler).await?;

        // ReviveHero(New) → job
        let revival_job_id = {
            let uow = uow_provider.begin().await?;
            let jobs = uow.jobs().list_by_player_id(player_id).await?;
            assert_eq!(jobs.len(), 1);
            let job = jobs[0].clone();
            assert_eq!(job.task.task_type, "HeroRevival");
            let job_id = job.id;
            uow.rollback().await?;
            job_id
        };

        {
            let uow = uow_provider.begin().await?;
            let job = uow.jobs().get_by_id(revival_job_id).await?;
            uow.rollback().await?;
            worker.process_jobs(&vec![job.clone()]).await?;
        }

        {
            let uow = uow_provider.begin().await?;
            let hero_repo = uow.heroes();
            let job_repo = uow.jobs();

            let hero = hero_repo.get_by_id(hero_id).await?;
            assert!(hero.is_alive());
            assert_eq!(hero.level, 0);
            assert_eq!(hero.experience, 0);
            assert_eq!(hero.strength_points, 0);
            assert_eq!(hero.off_bonus_points, 0);
            assert_eq!(hero.def_bonus_points, 0);
            assert_eq!(hero.regeneration_points, 0);
            assert_eq!(hero.resources_points, 0);
            assert_eq!(hero.unassigned_points, 5);
            assert_eq!(hero.village_id, village_id);

            let job = job_repo.get_by_id(revival_job_id).await?;
            assert_eq!(job.status, JobStatus::Completed);

            uow.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_hero_xp_levelup_and_revival() -> Result<()> {
        let (app, worker, uow_provider, config) = setup_app().await?;

        let uow = uow_provider.begin().await?;
        let village_repo = uow.villages();
        let hero_repo = uow.heroes();

        let (player_id, village_id, hero_id) = {
            let (player, mut village, _, some_hero) =
                setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], true).await?;
            let mut hero = some_hero.unwrap();

            let granary =
                Building::new(BuildingName::Granary, config.speed).at_level(20, config.speed)?;
            let warehouse =
                Building::new(BuildingName::Warehouse, config.speed).at_level(20, config.speed)?;
            village.add_building_at_slot(granary, 21)?;
            village.add_building_at_slot(warehouse, 20)?;

            village.store_resources(&ResourceGroup(100_000, 100_000, 100_000, 100_000));
            village_repo.save(&village).await?;

            // lv0, xp0, HP at  (così verifichiamo l'heal on level-up)
            hero.level = 0;
            hero.experience = 0;
            hero.health = 40;
            hero_repo.save(&hero).await?;

            uow.commit().await?;
            (player.id, village.id, hero.id)
        };

        // 2) Battle with enough XP to level-up hero (T3: threshold lv1 = 100 XP)
        {
            let uow = uow_provider.begin().await?;
            let hero_repo = uow.heroes();

            let mut hero = hero_repo.get_by_id(hero_id).await?;
            assert_eq!(hero.level, 0);
            assert_eq!(hero.health, 40);

            // simulate hero XP gaining
            let leveled = hero.gain_experience(150);
            assert!(leveled >= 1);
            assert!(hero.level >= 1);
            assert_eq!(hero.health, 100);

            let xp_after = hero.experience;
            assert!(xp_after >= 150);

            hero_repo.save(&hero).await?;
            uow.commit().await?;
        }

        // 3) Next battle with >= 90% losses → hero dies
        {
            let uow = uow_provider.begin().await?;
            let hero_repo = uow.heroes();

            let mut hero = hero_repo.get_by_id(hero_id).await?;
            assert!(hero.is_alive());

            hero.apply_battle_damage(0.95);
            assert!(!hero.is_alive());
            assert_eq!(hero.health, 0);

            hero_repo.save(&hero).await?;
            uow.commit().await?;
        }

        let cmd = ReviveHero {
            player_id,
            hero_id,
            village_id,
            reset: false,
        };
        let handler = ReviveHeroCommandHandler::new();
        app.execute(cmd, handler).await?;

        // 4) Revive hero
        let revival_job_id = {
            let uow = uow_provider.begin().await?;
            uow.commit().await?;

            let uow = uow_provider.begin().await?;
            let jobs = uow.jobs().list_by_player_id(player_id).await?;
            assert_eq!(jobs.len(), 1);
            let job = jobs[0].clone();
            assert_eq!(job.task.task_type, "HeroRevival");
            let job_id = job.id;

            let hero = uow.heroes().get_by_id(hero_id).await?;
            assert_eq!(hero.health, 0);

            uow.rollback().await?;
            job_id
        };

        // 5) Process HeroRevival job
        {
            let uow = uow_provider.begin().await?;
            let job = uow.jobs().get_by_id(revival_job_id).await?;
            uow.rollback().await?;
            worker.process_jobs(&vec![job.clone()]).await?;
        }

        // 6) Final state
        {
            let uow = uow_provider.begin().await?;
            let hero_repo = uow.heroes();
            let job_repo = uow.jobs();

            let hero = hero_repo.get_by_id(hero_id).await?;
            assert!(hero.is_alive());
            assert!(hero.level >= 1);
            assert!(hero.experience >= 150);
            assert_eq!(hero.health, 100);
            assert_eq!(hero.village_id, village_id);

            let job = job_repo.get_by_id(revival_job_id).await?;
            assert_eq!(job.status, JobStatus::Completed);

            uow.rollback().await?;
        }

        Ok(())
    }
}
