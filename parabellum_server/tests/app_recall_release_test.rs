mod test_utils;

#[cfg(test)]
pub mod tests {
    use parabellum_app::{
        command_handlers::{
            RecallTroopsCommandHandler, ReinforceVillageCommandHandler,
            ReleaseReinforcementsCommandHandler,
        },
        cqrs::commands::{RecallTroops, ReinforceVillage, ReleaseReinforcements},
        jobs::{JobStatus, tasks::ArmyReturnTask},
    };
    use parabellum_types::{Result, tribe::Tribe};

    use crate::test_utils::tests::{setup_app, setup_player_party};

    #[tokio::test]
    async fn test_recall_deployed_troops() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;
        let units_to_deploy = [100, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        // Setup deployer with troops
        let (deployer_player, deployer_village, deploying_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                units_to_deploy,
                false,
            )
            .await?
        };
        let original_home_army_id = deploying_army.id;

        // Setup target village (where troops will be deployed)
        let (_target_player, target_village, _target_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Step 1: Send reinforcements to target village
        let reinforce_command = ReinforceVillage {
            player_id: deployer_player.id,
            village_id: deployer_village.id,
            army_id: original_home_army_id,
            units: units_to_deploy,
            target_village_id: target_village.id,
            hero_id: None,
        };

        let handler = ReinforceVillageCommandHandler::new();
        app.execute(reinforce_command, handler).await?;

        // Process the reinforcement travel job
        let (reinforce_job, deployed_army_id) = {
            let uow_assert1 = uow_provider.tx().await?;
            let jobs = uow_assert1
                .jobs()
                .list_by_player_id(deployer_player.id)
                .await?;

            assert_eq!(jobs.len(), 1, "Should have 1 reinforcement job");
            let job = jobs.first().unwrap().clone();

            let payload: parabellum_app::jobs::tasks::ReinforcementTask =
                serde_json::from_value(job.task.data.clone())?;
            let deployed_id = payload.army_id;

            uow_assert1.rollback().await?;
            (job, deployed_id)
        };

        worker.process_jobs(&vec![reinforce_job]).await?;

        // Verify troops are deployed at target village
        {
            let uow_assert2 = uow_provider.tx().await?;

            let deployed_army = uow_assert2.armies().get_by_id(deployed_army_id).await?;
            assert_eq!(
                deployed_army.current_map_field_id,
                Some(target_village.id),
                "Army should be deployed at target village"
            );
            assert_eq!(
                deployed_army.village_id, deployer_village.id,
                "Army should still belong to deployer"
            );

            let target = uow_assert2.villages().get_by_id(target_village.id).await?;
            assert_eq!(
                target.reinforcements().len(),
                1,
                "Target village should have 1 reinforcement (from deployer's perspective)"
            );

            // From deployer's perspective, check deployed armies
            let deployer = uow_assert2
                .villages()
                .get_by_id(deployer_village.id)
                .await?;
            assert_eq!(
                deployer.deployed_armies().len(),
                1,
                "Deployer village should show 1 deployed army"
            );

            uow_assert2.rollback().await?;
        }

        // Step 2: Recall the deployed troops
        let recall_command = RecallTroops {
            player_id: deployer_player.id,
            village_id: deployer_village.id,
            army_id: deployed_army_id,
        };

        let recall_handler = RecallTroopsCommandHandler::new();
        app.execute(recall_command, recall_handler).await?;

        // Verify recall creates return job and army leaves immediately
        let recall_job = {
            let uow_assert3 = uow_provider.tx().await?;

            // Army should have left the target village immediately
            let recalled_army = uow_assert3.armies().get_by_id(deployed_army_id).await?;
            assert_eq!(
                recalled_army.current_map_field_id, None,
                "Army should have left target village immediately (current_map_field_id = None)"
            );

            // Target village should no longer show the army as reinforcement
            let target = uow_assert3.villages().get_by_id(target_village.id).await?;
            assert_eq!(
                target.reinforcements().len(),
                0,
                "Target village should have no reinforcements (army already left)"
            );

            // Deployer village should no longer show deployed armies
            let deployer = uow_assert3
                .villages()
                .get_by_id(deployer_village.id)
                .await?;
            assert_eq!(
                deployer.deployed_armies().len(),
                0,
                "Deployer village should have no deployed armies (army already left)"
            );

            // Should have a return job
            let jobs = uow_assert3
                .jobs()
                .list_by_player_id(deployer_player.id)
                .await?;
            assert_eq!(jobs.len(), 1, "Should have 1 recall/return job");

            let job = jobs.first().unwrap().clone();
            assert_eq!(job.task.task_type, "ArmyReturn");
            assert_eq!(job.status, JobStatus::Pending);

            let payload: ArmyReturnTask = serde_json::from_value(job.task.data.clone())?;
            assert_eq!(payload.army_id, deployed_army_id);
            assert_eq!(
                payload.destination_village_id, deployer_village.id as i32,
                "Army should be returning home"
            );

            uow_assert3.rollback().await?;
            job
        };

        // Step 3: Process the recall/return job
        worker.process_jobs(&vec![recall_job.clone()]).await?;

        // Verify troops returned home
        {
            let uow_assert4 = uow_provider.tx().await?;

            let return_job = uow_assert4.jobs().get_by_id(recall_job.id).await?;
            assert_eq!(return_job.status, JobStatus::Completed);

            // Deployed army should be deleted (merged back home)
            assert!(
                uow_assert4
                    .armies()
                    .get_by_id(deployed_army_id)
                    .await
                    .is_err(),
                "Deployed army should be deleted after returning home"
            );

            // Home village should have the army back
            let home_village = uow_assert4
                .villages()
                .get_by_id(deployer_village.id)
                .await?;
            assert!(
                home_village.army().is_some(),
                "Home village should have army back"
            );
            let home_army = home_village.army().unwrap();
            assert_eq!(home_army.units()[0], 100, "Should have all 100 troops back");

            uow_assert4.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_release_reinforcements() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;
        let units_to_send = [50, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        // Setup reinforcer with troops
        let (reinforcer_player, reinforcer_village, reinforcing_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                units_to_send,
                false,
            )
            .await?
        };
        let original_home_army_id = reinforcing_army.id;

        // Setup host village (where reinforcements will be sent)
        let (host_player, host_village, _host_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                [10, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Step 1: Send reinforcements to host village
        let reinforce_command = ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: original_home_army_id,
            units: units_to_send,
            target_village_id: host_village.id,
            hero_id: None,
        };

        let handler = ReinforceVillageCommandHandler::new();
        app.execute(reinforce_command, handler).await?;

        // Process the reinforcement travel job
        let (reinforce_job, deployed_army_id) = {
            let uow_assert1 = uow_provider.tx().await?;
            let jobs = uow_assert1
                .jobs()
                .list_by_player_id(reinforcer_player.id)
                .await?;

            assert_eq!(jobs.len(), 1, "Should have 1 reinforcement job");
            let job = jobs.first().unwrap().clone();

            let payload: parabellum_app::jobs::tasks::ReinforcementTask =
                serde_json::from_value(job.task.data.clone())?;
            let deployed_id = payload.army_id;

            uow_assert1.rollback().await?;
            (job, deployed_id)
        };

        worker.process_jobs(&vec![reinforce_job]).await?;

        // Verify reinforcements arrived at host village
        {
            let uow_assert2 = uow_provider.tx().await?;

            let reinforcement_army = uow_assert2.armies().get_by_id(deployed_army_id).await?;
            assert_eq!(
                reinforcement_army.current_map_field_id,
                Some(host_village.id),
                "Reinforcement should be at host village"
            );
            assert_eq!(
                reinforcement_army.village_id, reinforcer_village.id,
                "Reinforcement should belong to reinforcer"
            );

            let host = uow_assert2.villages().get_by_id(host_village.id).await?;
            assert_eq!(
                host.reinforcements().len(),
                1,
                "Host village should have 1 reinforcement"
            );

            uow_assert2.rollback().await?;
        }

        // Step 2: Host player releases the reinforcements
        let release_command = ReleaseReinforcements {
            player_id: host_player.id,
            village_id: host_village.id,
            source_village_id: reinforcer_village.id,
        };

        let release_handler = ReleaseReinforcementsCommandHandler::new();
        app.execute(release_command, release_handler).await?;

        // Verify release creates return job and reinforcement leaves immediately
        let release_job = {
            let uow_assert3 = uow_provider.tx().await?;

            // Reinforcement should have left the host village immediately
            let released_army = uow_assert3.armies().get_by_id(deployed_army_id).await?;
            assert_eq!(
                released_army.current_map_field_id, None,
                "Reinforcement should have left host village immediately"
            );

            // Host village should no longer show the reinforcement
            let host = uow_assert3.villages().get_by_id(host_village.id).await?;
            assert_eq!(
                host.reinforcements().len(),
                0,
                "Host village should have no reinforcements (army already left)"
            );

            // Should have a return job (assigned to reinforcer player)
            let jobs = uow_assert3
                .jobs()
                .list_by_player_id(reinforcer_player.id)
                .await?;
            assert_eq!(jobs.len(), 1, "Should have 1 release/return job");

            let job = jobs.first().unwrap().clone();
            assert_eq!(job.task.task_type, "ArmyReturn");
            assert_eq!(job.status, JobStatus::Pending);

            let payload: ArmyReturnTask = serde_json::from_value(job.task.data.clone())?;
            assert_eq!(payload.army_id, deployed_army_id);
            assert_eq!(
                payload.destination_village_id, reinforcer_village.id as i32,
                "Reinforcement should be returning to source"
            );

            uow_assert3.rollback().await?;
            job
        };

        // Step 3: Process the release/return job
        worker.process_jobs(&vec![release_job.clone()]).await?;

        // Verify reinforcements returned to source village
        {
            let uow_assert4 = uow_provider.tx().await?;

            let return_job = uow_assert4.jobs().get_by_id(release_job.id).await?;
            assert_eq!(return_job.status, JobStatus::Completed);

            // Deployed army should be deleted (merged back home)
            assert!(
                uow_assert4
                    .armies()
                    .get_by_id(deployed_army_id)
                    .await
                    .is_err(),
                "Deployed army should be deleted after returning home"
            );

            // Reinforcer village should have the army back
            let reinforcer = uow_assert4
                .villages()
                .get_by_id(reinforcer_village.id)
                .await?;
            assert!(
                reinforcer.army().is_some(),
                "Reinforcer village should have army back"
            );
            let home_army = reinforcer.army().unwrap();
            assert_eq!(home_army.units()[0], 50, "Should have all 50 troops back");

            uow_assert4.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_recall_unauthorized_player() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;

        // Setup deployer with troops
        let (deployer_player, deployer_village, deploying_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                [100, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Setup target village
        let (_target_player, target_village, _target_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Send reinforcements
        let reinforce_command = ReinforceVillage {
            player_id: deployer_player.id,
            village_id: deployer_village.id,
            army_id: deploying_army.id,
            units: [100, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: target_village.id,
            hero_id: None,
        };

        app.execute(reinforce_command, ReinforceVillageCommandHandler::new())
            .await?;

        let (reinforce_job, deployed_army_id) = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(deployer_player.id).await?;
            let job = jobs.first().unwrap().clone();
            let payload: parabellum_app::jobs::tasks::ReinforcementTask =
                serde_json::from_value(job.task.data.clone())?;
            uow.rollback().await?;
            (job, payload.army_id)
        };

        worker.process_jobs(&vec![reinforce_job]).await?;

        // Setup unauthorized player trying to recall
        let (unauthorized_player, unauthorized_village, _, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Teuton,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Try to recall someone else's troops - should fail
        let recall_command = RecallTroops {
            player_id: unauthorized_player.id,
            village_id: unauthorized_village.id,
            army_id: deployed_army_id,
        };

        let result = app
            .execute(recall_command, RecallTroopsCommandHandler::new())
            .await;

        assert!(
            result.is_err(),
            "Should not allow unauthorized player to recall troops"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_release_unauthorized_player() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;

        // Setup reinforcer with troops
        let (reinforcer_player, reinforcer_village, reinforcing_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                [50, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Setup host village
        let (_host_player, host_village, _host_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                [10, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Send reinforcements
        let reinforce_command = ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: reinforcing_army.id,
            units: [50, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: host_village.id,
            hero_id: None,
        };

        app.execute(reinforce_command, ReinforceVillageCommandHandler::new())
            .await?;

        let reinforce_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(reinforcer_player.id).await?;
            let job = jobs.first().unwrap().clone();
            uow.rollback().await?;
            job
        };

        worker.process_jobs(&vec![reinforce_job]).await?;

        // Setup unauthorized player trying to release
        let (unauthorized_player, unauthorized_village, _, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Teuton,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Try to release someone else's reinforcements - should fail
        let release_command = ReleaseReinforcements {
            player_id: unauthorized_player.id,
            village_id: unauthorized_village.id,
            source_village_id: reinforcer_village.id,
        };

        let result = app
            .execute(release_command, ReleaseReinforcementsCommandHandler::new())
            .await;

        assert!(
            result.is_err(),
            "Should not allow unauthorized player to release reinforcements"
        );

        Ok(())
    }
}
