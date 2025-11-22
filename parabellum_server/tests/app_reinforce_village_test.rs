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
            uow_assert2.rollback().await?;
        }
        Ok(())
    }
}
