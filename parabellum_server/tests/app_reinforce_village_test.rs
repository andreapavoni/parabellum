mod test_utils;

#[cfg(test)]
pub mod tests {
    use parabellum_app::{
        command_handlers::ReinforceVillageCommandHandler,
        cqrs::commands::ReinforceVillage,
        jobs::{JobStatus, tasks::ReinforcementTask},
    };
    use parabellum_types::Result;
    use parabellum_types::tribe::Tribe;

    use super::test_utils::tests::setup_player_party;
    use crate::test_utils::tests::setup_app;

    #[tokio::test]
    async fn test_reinforce_village() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;
        let units_to_send = [100, 0, 0, 0, 0, 0, 0, 0, 0, 0];

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

        let command = ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: original_home_army_id,
            units: units_to_send,
            target_village_id: target_village.id,
            hero_id: None,
        };

        let handler = ReinforceVillageCommandHandler::new();
        app.execute(command, handler).await?;

        let (reinforce_job, deployed_army_id) = {
            let uow_assert1 = uow_provider.tx().await?;
            let jobs = uow_assert1
                .jobs()
                .list_by_player_id(reinforcer_player.id)
                .await?;

            assert_eq!(
                jobs.len(),
                1,
                "Should have 1 job in the queue, got {}.",
                jobs.len()
            );
            let job = jobs.first().unwrap().clone();

            assert_eq!(job.status, JobStatus::Pending);
            assert_eq!(job.task.task_type, "Reinforcement");

            let payload: ReinforcementTask = serde_json::from_value(job.task.data.clone())?;
            assert_ne!(
                payload.army_id, original_home_army_id,
                "Deployed army ID should be new"
            );
            assert_eq!(payload.village_id, target_village.id as i32);

            let home_village = uow_assert1
                .villages()
                .get_by_id(reinforcer_village.id)
                .await?;
            assert!(
                home_village.army().is_none(),
                "Home village army should be None"
            );
            assert!(
                uow_assert1
                    .armies()
                    .get_by_id(original_home_army_id)
                    .await
                    .is_err(),
                "Original home army should be deleted"
            );
            assert!(
                uow_assert1
                    .armies()
                    .get_by_id(payload.army_id)
                    .await
                    .is_ok(),
                "Deployed army should exist"
            );

            uow_assert1.rollback().await?;
            (job, payload.army_id)
        };

        worker.process_jobs(&vec![reinforce_job.clone()]).await?;
        {
            let uow_assert2 = uow_provider.tx().await?;

            let original_job = uow_assert2.jobs().get_by_id(reinforce_job.id).await?;
            assert_eq!(original_job.status, JobStatus::Completed);

            let pending_jobs = uow_assert2
                .jobs()
                .list_by_player_id(reinforcer_player.id)
                .await?;
            assert_eq!(pending_jobs.len(), 0, "There shouldn't be return jobs");

            let final_army = uow_assert2.armies().get_by_id(deployed_army_id).await?;
            assert_eq!(
                final_army.current_map_field_id,
                Some(target_village.id),
                "Reinforcements should be in target village"
            );

            let final_target_village = uow_assert2.villages().get_by_id(target_village.id).await?;
            let reinforcer_village = uow_assert2
                .villages()
                .get_by_id(reinforcer_village.id)
                .await?;

            assert_eq!(final_target_village.reinforcements().len(), 1);
            assert_eq!(
                final_target_village.reinforcements()[0].id,
                deployed_army_id, // <-- Check for deployed_army_id
                "Target village should have reinforcements"
            );
            assert!(
                reinforcer_village.army().is_none(),
                "Reinforcer village shouldn't have army at home"
            );

            // Verify reinforcement report was created
            let reports = uow_assert2
                .reports()
                .list_for_player(reinforcer_player.id, 10)
                .await?;
            assert_eq!(
                reports.len(),
                1,
                "Should have created 1 reinforcement report"
            );

            let report = &reports[0];
            assert_eq!(report.report_type, "reinforcement");

            // Verify report payload
            if let parabellum_types::reports::ReportPayload::Reinforcement(ref payload) =
                report.payload
            {
                assert_eq!(payload.sender_village, reinforcer_village.name);
                assert_eq!(payload.receiver_village, final_target_village.name);
                assert_eq!(payload.units, units_to_send);
                assert_eq!(payload.tribe, Tribe::Roman);
            } else {
                panic!("Expected Reinforcement report payload");
            }

            uow_assert2.rollback().await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_reinforcements_from_same_village_are_merged() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;

        // Setup reinforcer with plenty of troops
        let (reinforcer_player, reinforcer_village, reinforcing_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                [200, 100, 50, 30, 0, 0, 0, 0, 0, 0], // Multiple unit types
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

        // First reinforcement: send 50 legionnaires
        let first_units = [50, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let command1 = ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: reinforcing_army.id,
            units: first_units,
            target_village_id: target_village.id,
            hero_id: None,
        };

        app.execute(command1, ReinforceVillageCommandHandler::new())
            .await?;

        let first_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(reinforcer_player.id).await?;
            assert_eq!(jobs.len(), 1);
            let job = jobs[0].clone();
            uow.rollback().await?;
            job
        };

        // Process first reinforcement
        worker.process_jobs(&vec![first_job]).await?;

        // Verify first reinforcement arrived
        let first_reinforcement_id = {
            let uow = uow_provider.tx().await?;
            let village = uow.villages().get_by_id(target_village.id).await?;
            assert_eq!(
                village.reinforcements().len(),
                1,
                "Should have 1 reinforcement army"
            );
            assert_eq!(
                village.reinforcements()[0].units()[0],
                50,
                "Should have 50 legionnaires"
            );
            let id = village.reinforcements()[0].id;
            uow.rollback().await?;
            id
        };

        // Get updated home army ID after first reinforcement
        let home_army_id_after_first = {
            let uow = uow_provider.tx().await?;
            let home_village = uow.villages().get_by_id(reinforcer_village.id).await?;
            let army_id = home_village
                .army()
                .expect("Home village should have remaining army")
                .id;
            uow.rollback().await?;
            army_id
        };

        // Second reinforcement: send 30 praetorians and 20 legionnaires
        let second_units = [20, 30, 0, 0, 0, 0, 0, 0, 0, 0];
        let command2 = ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: home_army_id_after_first,
            units: second_units,
            target_village_id: target_village.id,
            hero_id: None,
        };

        app.execute(command2, ReinforceVillageCommandHandler::new())
            .await?;

        let second_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(reinforcer_player.id).await?;
            assert_eq!(jobs.len(), 1, "Should have 1 pending job");
            let job = jobs[0].clone();
            uow.rollback().await?;
            job
        };

        // Process second reinforcement
        worker.process_jobs(&vec![second_job]).await?;

        // Verify reinforcements were merged
        {
            let uow = uow_provider.tx().await?;
            let village = uow.villages().get_by_id(target_village.id).await?;

            // Should still have only 1 reinforcement army (merged)
            assert_eq!(
                village.reinforcements().len(),
                1,
                "Should have exactly 1 reinforcement army (merged), found {}",
                village.reinforcements().len()
            );

            let merged_reinforcement = &village.reinforcements()[0];

            // Verify the merged army has combined units
            assert_eq!(
                merged_reinforcement.units()[0],
                70, // 50 + 20 legionnaires
                "Should have 70 legionnaires (50 + 20)"
            );
            assert_eq!(
                merged_reinforcement.units()[1],
                30, // 30 praetorians
                "Should have 30 praetorians"
            );

            // Verify the ID is the same as the first reinforcement
            assert_eq!(
                merged_reinforcement.id, first_reinforcement_id,
                "Reinforcement ID should remain the same after merge"
            );

            // Verify home village still has remaining troops
            let home_village = uow.villages().get_by_id(reinforcer_village.id).await?;
            assert!(
                home_village.army().is_some(),
                "Home village should still have remaining army"
            );
            let home_army = home_village.army().unwrap();
            assert_eq!(
                home_army.units()[0],
                130, // 200 - 50 - 20
                "Home should have 130 legionnaires remaining"
            );
            assert_eq!(
                home_army.units()[1],
                70, // 100 - 30
                "Home should have 70 praetorians remaining"
            );

            uow.rollback().await?;
        }

        Ok(())
    }
}
