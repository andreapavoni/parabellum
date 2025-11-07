mod test_utils;

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use parabellum_app::{
        command_handlers::SendResourcesCommandHandler,
        config::Config,
        cqrs::{CommandHandler, commands::SendResources},
        job_registry::AppJobRegistry,
        jobs::{
            JobStatus,
            tasks::{MerchantGoingTask, MerchantReturnTask},
            worker::JobWorker,
        },
        uow::UnitOfWorkProvider,
    };
    use parabellum_core::{ApplicationError, GameError, Result};
    use parabellum_db::establish_test_connection_pool;
    use parabellum_game::{
        models::{buildings::Building, village::Village},
        test_utils::{
            PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions, player_factory,
            valley_factory, village_factory,
        },
    };
    use parabellum_types::{
        buildings::BuildingName,
        common::{Player, ResourceGroup},
        map::Position,
        tribe::Tribe,
    };

    use super::test_utils::tests::TestUnitOfWorkProvider;

    /// Helper to set 2 players + 2 villages.
    async fn setup_test_env() -> Result<
        (
            Arc<dyn UnitOfWorkProvider>,
            Arc<Config>,
            Arc<AppJobRegistry>,
            Player,  // Sender Player
            Village, // Sender Village
            Village, // Target Village
        ),
        ApplicationError,
    > {
        let pool = establish_test_connection_pool().await?;
        let master_tx = pool.begin().await.unwrap();
        let master_tx_arc = Arc::new(Mutex::new(master_tx));
        let uow_provider: Arc<dyn UnitOfWorkProvider> =
            Arc::new(TestUnitOfWorkProvider::new(master_tx_arc.clone()));
        let app_registry = Arc::new(AppJobRegistry::new());
        let config = Arc::new(Config::from_env());

        let (player_a, village_a, village_b) = {
            let uow = uow_provider.begin().await?;

            // Player A (Sender) - Gauls (Capacity 750)
            let player_a = player_factory(PlayerFactoryOptions {
                tribe: Some(Tribe::Gaul),
                ..Default::default()
            });
            uow.players().save(&player_a).await?;

            let valley_a = valley_factory(ValleyFactoryOptions {
                position: Some(Position { x: 1, y: 1 }),
                ..Default::default()
            });
            let mut village_a = village_factory(VillageFactoryOptions {
                player: Some(player_a.clone()),
                valley: Some(valley_a),
                ..Default::default()
            });

            let granary = Building::new(BuildingName::Granary, config.speed)
                .at_level(10, config.speed)
                .unwrap();
            village_a.add_building_at_slot(granary, 23).unwrap();

            let warehouse = Building::new(BuildingName::Warehouse, config.speed)
                .at_level(10, config.speed)
                .unwrap();
            village_a.add_building_at_slot(warehouse, 24).unwrap();

            let marketplace = Building::new(BuildingName::Marketplace, config.speed)
                .at_level(10, config.speed)?;
            village_a.add_building_at_slot(marketplace, 25)?;

            village_a
                .stocks
                .store_resources(ResourceGroup(5000, 5000, 5000, 5000));
            village_a.update_state();
            uow.villages().save(&village_a).await?;

            // Player B (Receiver)
            let player_b = player_factory(PlayerFactoryOptions {
                tribe: Some(Tribe::Roman),
                ..Default::default()
            });
            uow.players().save(&player_b).await?;

            let valley_b = valley_factory(ValleyFactoryOptions {
                position: Some(Position { x: 2, y: 2 }),
                ..Default::default()
            });
            let mut village_b = village_factory(VillageFactoryOptions {
                player: Some(player_b.clone()),
                valley: Some(valley_b),
                ..Default::default()
            });
            let granary = Building::new(BuildingName::Granary, config.speed)
                .at_level(10, config.speed)
                .unwrap();
            village_b.add_building_at_slot(granary, 23).unwrap();

            let warehouse = Building::new(BuildingName::Warehouse, config.speed)
                .at_level(10, config.speed)
                .unwrap();
            village_b.add_building_at_slot(warehouse, 24).unwrap();

            uow.villages().save(&village_b).await?;
            uow.commit().await?;
            (player_a, village_a, village_b)
        };

        Ok((
            uow_provider,
            config,
            app_registry,
            player_a,
            village_a,
            village_b,
        ))
    }

    #[tokio::test]
    async fn test_full_merchant_flow() -> Result<()> {
        let (uow_provider, config, app_registry, player_a, village_a, village_b) =
            setup_test_env().await?;

        let resources_to_send = ResourceGroup(1000, 500, 0, 0); // 1500
        let merchants_needed = 2; // 1500 / 750 (Gaul capacity) = 2
        let initial_sender_lumber = village_a.stocks.lumber;
        let initial_target_lumber = village_b.stocks.lumber;

        let command = SendResources {
            player_id: player_a.id,
            village_id: village_a.id,
            target_village_id: village_b.id,
            resources: resources_to_send.clone(),
        };

        {
            let uow_cmd = uow_provider.begin().await?;
            let handler = SendResourcesCommandHandler::new();
            handler.handle(command, &uow_cmd, &config).await?;
            uow_cmd.commit().await?;
        }

        let going_job = {
            let uow_assert1 = uow_provider.begin().await?;

            let sender_village = uow_assert1.villages().get_by_id(village_a.id).await?;
            assert_eq!(
                sender_village.stocks.lumber,
                initial_sender_lumber - resources_to_send.0,
                "No resources withdrawn from sender stocks"
            );
            assert_eq!(
                sender_village.stocks.clay,
                village_a.stocks.clay - resources_to_send.1
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

        let worker = Arc::new(JobWorker::new(
            uow_provider.clone(),
            app_registry.clone(),
            config.clone(),
        ));
        worker.process_jobs(&vec![going_job.clone()]).await?;

        let return_job = {
            let uow_assert2 = uow_provider.begin().await?;

            let target_village = uow_assert2.villages().get_by_id(village_b.id).await?;
            assert_eq!(
                target_village.stocks.lumber,
                initial_target_lumber + resources_to_send.0,
                "Expected to have {} lumber, got {}",
                initial_target_lumber + resources_to_send.0,
                target_village.stocks.lumber,
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
            let uow_assert3 = uow_provider.begin().await?;

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
        let (uow_provider, config, _registry, player_a, village_a, village_b) =
            setup_test_env().await?;

        {
            let uow = uow_provider.begin().await?;
            let mut v = uow.villages().get_by_id(village_a.id).await?;
            v.set_building_level_at_slot(25, 1, config.speed)?;
            v.update_state();
            uow.villages().save(&v).await?;
            uow.commit().await?;
        }

        let command = SendResources {
            player_id: player_a.id,
            village_id: village_a.id,
            target_village_id: village_b.id,
            resources: ResourceGroup(1500, 0, 0, 0),
        };

        let uow_cmd = uow_provider.begin().await?;
        let handler = SendResourcesCommandHandler::new();
        let result = handler.handle(command, &uow_cmd, &config).await;

        assert!(result.is_err(), "Expected failure, got success");
        if let Err(ApplicationError::Game(GameError::NotEnoughMerchants)) = result {
            // Success
        } else {
            panic!("Wrong failure message: {:?}", result.err());
        }

        let jobs = uow_cmd.jobs().list_by_player_id(player_a.id).await?;
        assert_eq!(jobs.len(), 0);

        uow_cmd.rollback().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_send_resources_fail_not_enough_resources() -> Result<()> {
        let (uow_provider, config, _registry, player_a, village_a, village_b) =
            setup_test_env().await?;

        // Tento di inviare 5001 lumber (ne ho 5000)
        let command = SendResources {
            player_id: player_a.id,
            village_id: village_a.id,
            target_village_id: village_b.id,
            resources: ResourceGroup(5001, 0, 0, 0),
        };

        let uow_cmd = uow_provider.begin().await?;
        let handler = SendResourcesCommandHandler::new();
        let result = handler.handle(command, &uow_cmd, &config).await;

        // Verifica l'errore
        assert!(result.is_err(), "Il comando doveva fallire");
        if let Err(ApplicationError::Game(GameError::NotEnoughResources)) = result {
            // Successo
        } else {
            panic!("Errore non corretto: {:?}", result.err());
        }

        uow_cmd.rollback().await?;
        Ok(())
    }
}
