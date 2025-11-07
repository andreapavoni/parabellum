mod test_utils;

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;
    use tokio::sync::Mutex;

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
    use parabellum_types::{map::Position, tribe::Tribe};

    use super::test_utils::tests::TestUnitOfWorkProvider;
    use super::test_utils::tests::setup_player_party;

    #[tokio::test]
    async fn test_full_reinforce_flow() -> Result<()> {
        let pool = establish_test_connection_pool().await.unwrap();
        let master_tx = pool.begin().await.unwrap();
        let master_tx_arc = Arc::new(Mutex::new(master_tx));
        let app_registry = Arc::new(AppJobRegistry::new());
        let config = Arc::new(Config::from_env());

        let uow_provider: Arc<dyn UnitOfWorkProvider> =
            Arc::new(TestUnitOfWorkProvider::new(master_tx_arc.clone()));

        let (reinforcer_player, reinforcer_village, reinforcing_army) = {
            setup_player_party(
                uow_provider.clone(),
                Position { x: 10, y: 10 },
                Tribe::Roman,
                [100, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            )
            .await?
        };

        let (_target_player, target_village, _target_army) = {
            setup_player_party(
                uow_provider.clone(),
                Position { x: 20, y: 20 },
                Tribe::Gaul,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            )
            .await?
        };

        let command = ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: reinforcing_army.id,
            target_village_id: target_village.id,
        };

        {
            let uow_cmd = uow_provider.begin().await?;
            let handler = ReinforceVillageCommandHandler::new();
            handler.handle(command, &uow_cmd, &config).await?;
            uow_cmd.commit().await?;
        };

        let reinforce_job = {
            let uow_assert1 = uow_provider.begin().await?;
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
            assert_eq!(payload.army_id, reinforcing_army.id);
            assert_eq!(payload.village_id, target_village.id as i32);

            uow_assert1.rollback().await?;
            job
        };

        let worker = Arc::new(JobWorker::new(
            uow_provider.clone(),
            app_registry,
            config.clone(),
        ));
        worker.process_jobs(&vec![reinforce_job.clone()]).await?;

        {
            let uow_assert2 = uow_provider.begin().await?;

            let original_job = uow_assert2.jobs().get_by_id(reinforce_job.id).await?;
            assert_eq!(original_job.status, JobStatus::Completed);

            let pending_jobs = uow_assert2
                .jobs()
                .list_by_player_id(reinforcer_player.id)
                .await?;
            assert_eq!(pending_jobs.len(), 0, "There shouldn't be return jobs");

            let final_army = uow_assert2.armies().get_by_id(reinforcing_army.id).await?;
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

            assert_eq!(final_target_village.reinforcements.len(), 1);
            assert_eq!(
                final_target_village.reinforcements[0].id, reinforcing_army.id,
                "Target village should have reinforcements"
            );
            assert_eq!(
                reinforcer_village.army.is_none(),
                true,
                "Reinforcer village shouldn't have army at home"
            );

            uow_assert2.rollback().await?;
        }

        Ok(())
    }
}
