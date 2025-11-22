mod test_utils;

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use parabellum_app::{
        app::AppBus,
        command_handlers::SendResourcesCommandHandler,
        config::Config,
        cqrs::commands::SendResources,
        jobs::{
            JobStatus,
            tasks::{MerchantGoingTask, MerchantReturnTask},
            worker::JobWorker,
        },
        uow::UnitOfWorkProvider,
    };
    use parabellum_types::{errors::{ApplicationError, GameError}, Result};

    use parabellum_game::models::{buildings::Building, village::Village};
    use parabellum_types::{
        buildings::BuildingName,
        common::{Player, ResourceGroup},
        tribe::Tribe,
    };

    use crate::test_utils::tests::{setup_app, setup_player_party};

    async fn setup_test_env() -> Result<
        (
            Arc<dyn UnitOfWorkProvider>,
            Arc<Config>,
            AppBus,         // App Bus
            Arc<JobWorker>, // JobWorker
            Player,         // Sender Player
            Village,        // Sender Village
            Village,        // Target Village
        ),
        ApplicationError,
    > {
        let (app, worker, uow_provider, config) = setup_app(false).await?;

        let (player_a, mut village_a, _, _, _) = setup_player_party(
            uow_provider.clone(),
            None,
            Tribe::Teuton,
            [20, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            false,
        )
        .await?;
        let (_, mut village_b, _, _, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        let uow = uow_provider.tx().await?;

        let granary =
            Building::new(BuildingName::Granary, config.speed).at_level(10, config.speed)?;
        village_a.add_building_at_slot(granary, 23).unwrap();

        let warehouse =
            Building::new(BuildingName::Warehouse, config.speed).at_level(10, config.speed)?;
        village_a.add_building_at_slot(warehouse, 24).unwrap();

        let marketplace =
            Building::new(BuildingName::Marketplace, config.speed).at_level(10, config.speed)?;
        village_a.add_building_at_slot(marketplace, 25)?;
        village_a.store_resources(&ResourceGroup(5000, 5000, 5000, 5000));

        let granary =
            Building::new(BuildingName::Granary, config.speed).at_level(10, config.speed)?;
        village_b.add_building_at_slot(granary, 23).unwrap();

        let warehouse =
            Building::new(BuildingName::Warehouse, config.speed).at_level(10, config.speed)?;
        village_b.add_building_at_slot(warehouse, 24).unwrap();

        uow.villages().save(&village_a).await?;
        uow.villages().save(&village_b).await?;
        uow.commit().await?;

        Ok((
            uow_provider,
            config,
            app,
            worker,
            player_a,
            village_a,
            village_b,
        ))
    }

    #[tokio::test]
    async fn test_full_merchant_exchange() -> Result<()> {
        let (uow_provider, _config, app, worker, player_a, village_a, village_b) =
            setup_test_env().await?;

        let resources_to_send = ResourceGroup(1000, 500, 0, 0); // 1500
        let merchants_needed = 2; // 1500 / 750 (Gaul capacity) = 2
        let initial_sender_lumber = village_a.stored_resources().lumber();
        let initial_target_lumber = village_b.stored_resources().lumber();

        let command = SendResources {
            player_id: player_a.id,
            village_id: village_a.id,
            target_village_id: village_b.id,
            resources: resources_to_send.clone(),
        };
        let handler = SendResourcesCommandHandler::new();
        app.execute(command, handler).await?;

        let going_job = {
            let uow_assert1 = uow_provider.tx().await?;

            let sender_village = uow_assert1.villages().get_by_id(village_a.id).await?;
            assert_eq!(
                sender_village.stored_resources().lumber(),
                initial_sender_lumber - resources_to_send.0,
                "No resources withdrawn from sender stocks"
            );
            assert_eq!(
                sender_village.stored_resources().clay(),
                village_a.stored_resources().clay() - resources_to_send.1
            );

            assert_eq!(sender_village.total_merchants, 10);
            assert_eq!(
                sender_village.busy_merchants, merchants_needed as u8,
                "Expected {} busy merchants, got {}",
                sender_village.busy_merchants, merchants_needed
            );
            assert_eq!(
                sender_village.get_available_merchants(),
                10 - merchants_needed as u8
            );

            let jobs = uow_assert1.jobs().list_by_player_id(player_a.id).await?;
            assert_eq!(jobs.len(), 1, "There should be only a MerchantGoing job");

            let job = jobs.first().unwrap().clone();
            assert_eq!(job.task.task_type, "MerchantGoing");
            let payload: MerchantGoingTask = serde_json::from_value(job.task.data.clone())?;
            assert_eq!(payload.merchants_used, merchants_needed as u8);
            assert_eq!(payload.destination_village_id, village_b.id);
            assert!(payload.travel_time_secs > 0);

            uow_assert1.rollback().await?;
            job
        };

        worker.process_jobs(&vec![going_job.clone()]).await?;

        let return_job = {
            let uow_assert2 = uow_provider.tx().await?;

            let target_village = uow_assert2.villages().get_by_id(village_b.id).await?;
            assert_eq!(
                target_village.stored_resources().lumber(),
                initial_target_lumber + resources_to_send.lumber(),
                "Expected to have {} lumber, got {}",
                initial_target_lumber + resources_to_send.lumber(),
                target_village.stored_resources().lumber(),
            );

            let original_job = uow_assert2.jobs().get_by_id(going_job.id).await?;
            assert_eq!(original_job.status, JobStatus::Completed);

            let jobs = uow_assert2.jobs().list_by_player_id(player_a.id).await?;
            assert_eq!(jobs.len(), 1, "There should be only a MerchantReturn job");

            let job = jobs.first().unwrap().clone();
            assert_eq!(job.task.task_type, "MerchantReturn");

            let payload: MerchantReturnTask = serde_json::from_value(job.task.data.clone())?;
            assert_eq!(payload.merchants_used, merchants_needed as u8);

            let sender_village = uow_assert2.villages().get_by_id(village_a.id).await?;
            assert_eq!(sender_village.busy_merchants, merchants_needed as u8);
            assert_eq!(
                sender_village.get_available_merchants(),
                10 - merchants_needed as u8
            );

            uow_assert2.rollback().await?;
            job
        };

        worker.process_jobs(&vec![return_job.clone()]).await?;
        {
            let uow_assert3 = uow_provider.tx().await?;

            let original_job = uow_assert3.jobs().get_by_id(return_job.id).await?;
            assert_eq!(original_job.status, JobStatus::Completed);

            let jobs = uow_assert3.jobs().list_by_player_id(player_a.id).await?;
            assert_eq!(jobs.len(), 0, "Expected 0 pending jobs, got {}", jobs.len());

            // Controlla che i mercanti siano tornati disponibili
            let sender_village = uow_assert3.villages().get_by_id(village_a.id).await?;
            assert_eq!(sender_village.total_merchants, 10);
            assert_eq!(
                sender_village.busy_merchants, 0,
                "Expected 0 busy merchants, got {}",
                sender_village.busy_merchants
            );
            assert_eq!(sender_village.get_available_merchants(), 10);

            uow_assert3.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_send_resources_fail_not_enough_merchants() -> Result<()> {
        let (uow_provider, config, app, _worker, player_a, village_a, village_b) =
            setup_test_env().await?;

        {
            let uow = uow_provider.tx().await?;
            let mut v = uow.villages().get_by_id(village_a.id).await?;
            v.set_building_level_at_slot(25, 1, config.speed)?;
            uow.villages().save(&v).await?;
            uow.commit().await?;
        }

        let uow_cmd = uow_provider.tx().await?;
        let command = SendResources {
            player_id: player_a.id,
            village_id: village_a.id,
            target_village_id: village_b.id,
            resources: ResourceGroup(1500, 0, 0, 0),
        };
        let handler = SendResourcesCommandHandler::new();
        let result = app.execute(command, handler).await;
        assert!(result.is_err(), "Expected failure, got success");

        if let Err(ApplicationError::Game(GameError::NotEnoughMerchants)) = result {
            // Expected
        } else {
            panic!(
                "Wrong failure message: {:?}",
                result.err().unwrap().to_string()
            );
        }

        let jobs = uow_cmd.jobs().list_by_player_id(player_a.id).await?;
        assert_eq!(jobs.len(), 0);

        uow_cmd.rollback().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_send_resources_fail_not_enough_resources() -> Result<()> {
        let (_uow_provider, _config, app, _worker, player_a, village_a, village_b) =
            setup_test_env().await?;

        // Sending 5001 lumber while having 5000)
        let command = SendResources {
            player_id: player_a.id,
            village_id: village_a.id,
            target_village_id: village_b.id,
            resources: ResourceGroup(5001, 0, 0, 0),
        };
        let handler = SendResourcesCommandHandler::new();
        let result = app.execute(command, handler).await;

        // Verifica l'errore
        assert!(result.is_err(), "Expected failure, got success");
        if let Err(ApplicationError::Game(GameError::NotEnoughResources)) = result {
            // Successo
        } else {
            panic!("Wrong failure message: {:?}", result.err());
        }
        Ok(())
    }
}
