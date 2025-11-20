mod test_utils;

#[cfg(test)]
pub mod tests {
    use parabellum_app::{
        command_handlers::ScoutVillageCommandHandler,
        cqrs::commands::ScoutVillage,
        jobs::{
            JobStatus,
            tasks::{ArmyReturnTask, ScoutTask},
        },
    };
    use parabellum_core::Result;
    use parabellum_game::battle::ScoutingTarget;
    use parabellum_types::tribe::Tribe;

    use super::test_utils::tests::setup_player_party;
    use crate::test_utils::tests::setup_app;

    #[tokio::test]
    async fn test_scout_village() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;
        let scout_units = [0, 0, 0, 10, 0, 0, 0, 0, 0, 0]; // 10 Equites Legati (index 3)
        let (scout_player, scout_village, scout_army, _, _) = {
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, scout_units, false).await?
        };
        let original_home_army_id = scout_army.id;
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

        let command = ScoutVillage {
            player_id: scout_player.id,
            village_id: scout_village.id,
            army_id: original_home_army_id,
            target_village_id: target_village.id,
            target: ScoutingTarget::Resources,
            units: scout_units,
        };

        let handler = ScoutVillageCommandHandler::new();
        app.execute(command, handler).await?;

        let (scout_job, deployed_army_id) = {
            let uow_assert1 = uow_provider.begin().await?;
            let jobs = uow_assert1
                .jobs()
                .list_by_player_id(scout_player.id)
                .await?;

            assert_eq!(jobs.len(), 1, "Should have 1 job in the queue");
            let job = jobs.first().unwrap().clone();

            assert_eq!(job.status, JobStatus::Pending);
            assert_eq!(job.task.task_type, "Scout");

            let payload: ScoutTask = serde_json::from_value(job.task.data.clone())?;
            assert_ne!(
                payload.army_id, original_home_army_id,
                "Deployed army ID should be new"
            );
            assert_eq!(payload.target, ScoutingTarget::Resources);

            let scout_village = uow_assert1.villages().get_by_id(scout_village.id).await?;

            assert!(
                uow_assert1
                    .armies()
                    .get_by_id(original_home_army_id)
                    .await
                    .is_err(),
                "Initial home army should be removed",
            );

            assert!(
                uow_assert1
                    .armies()
                    .get_by_id(payload.army_id)
                    .await
                    .is_ok(),
                "Deployed scout army should exist",
            );

            assert!(
                scout_village.army().is_none(),
                "Scout village shouldn't have army at home (all troops sent)"
            );

            uow_assert1.rollback().await?;
            (job, payload.army_id)
        };

        worker.process_jobs(&vec![scout_job.clone()]).await?;

        let return_job = {
            let uow_assert2 = uow_provider.begin().await?;

            let original_job = uow_assert2.jobs().get_by_id(scout_job.id).await?;
            assert_eq!(original_job.status, JobStatus::Completed);

            let pending_jobs = uow_assert2
                .jobs()
                .list_by_player_id(scout_player.id)
                .await?;
            assert_eq!(pending_jobs.len(), 1, "Should have 1 return job.");

            let job = pending_jobs.first().unwrap().clone();
            assert_eq!(job.task.task_type, "ArmyReturn");

            let payload: ArmyReturnTask = serde_json::from_value(job.task.data.clone())?;
            assert_eq!(payload.army_id, deployed_army_id);
            assert_eq!(payload.resources.total(), 0, "Scouts don't carry a bounty");

            let army_status = uow_assert2.armies().get_by_id(deployed_army_id).await?;
            assert_eq!(army_status.units()[3], 10, "Scouts should have survived");
            assert!(
                uow_assert2
                    .armies()
                    .get_by_id(original_home_army_id)
                    .await
                    .is_err()
            );

            uow_assert2.rollback().await?;
            job
        };

        worker.process_jobs(&vec![return_job.clone()]).await?;
        {
            let uow_assert3 = uow_provider.begin().await?;
            let original_job = uow_assert3.jobs().get_by_id(return_job.id).await?;
            assert_eq!(
                original_job.status,
                JobStatus::Completed,
                "Return job should be completed"
            );

            let pending_jobs = uow_assert3
                .jobs()
                .list_by_player_id(scout_player.id)
                .await?;
            assert_eq!(pending_jobs.len(), 0, "Shouldn't be any jobs");

            let army_check = uow_assert3.armies().get_by_id(deployed_army_id).await;
            assert!(
                army_check.is_err(),
                "Moving army should be deleted after returning"
            );

            let home_village = uow_assert3.villages().get_by_id(scout_village.id).await?;

            assert!(
                home_village.army().is_some(),
                "Army should be returned at home"
            );
            let home_army = home_village.army().unwrap();
            assert_eq!(
                home_army.units()[3],
                10,
                "Expected 10 scouts at home, got {}",
                home_army.units()[3]
            );

            assert_ne!(home_army.id, deployed_army_id);
            assert_ne!(
                home_army.id, original_home_army_id,
                "Home army has a new ID after merge"
            );

            uow_assert3.rollback().await?;
        }

        Ok(())
    }
}
