use crate::{
    command_handlers::helpers::calculate_merchants_needed,
    config::Config,
    cqrs::{CommandHandler, commands::SendResources},
    jobs::{Job, JobPayload, tasks::MerchantGoingTask},
    uow::UnitOfWork,
};
use parabellum_types::{
    Result,
    errors::{ApplicationError, GameError},
};
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use std::sync::Arc;

pub struct SendResourcesCommandHandler {}

impl Default for SendResourcesCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SendResourcesCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<SendResources> for SendResourcesCommandHandler {
    async fn handle(
        &self,
        command: SendResources,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<()> {
        let village_repo = uow.villages();
        let job_repo = uow.jobs();

        let mut sender_village = village_repo.get_by_id(command.village_id).await?;
        let target_village = village_repo.get_by_id(command.target_village_id).await?;

        if sender_village.total_merchants == 0 {
            return Err(ApplicationError::Game(
                GameError::BuildingRequirementsNotMet {
                    building: BuildingName::Marketplace,
                    level: 1,
                },
            ));
        }

        let resources_to_send: ResourceGroup = command.resources;
        if resources_to_send.total() == 0 {
            return Ok(());
        }
        sender_village.deduct_resources(&resources_to_send)?;

        let merchants_needed =
            calculate_merchants_needed(&sender_village.tribe, resources_to_send.total())?;

        if sender_village.available_merchants() < merchants_needed {
            return Err(ApplicationError::Game(GameError::NotEnoughMerchants));
        }

        village_repo.save(&sender_village).await?;

        let merchant_stats = sender_village.tribe.merchant_stats();
        let travel_time_secs = sender_village.position.calculate_travel_time_secs(
            target_village.position,
            merchant_stats.speed,
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        let going_payload = MerchantGoingTask {
            origin_village_id: sender_village.id,
            destination_village_id: command.target_village_id,
            resources: resources_to_send,
            merchants_used: merchants_needed,
            travel_time_secs,
        };
        let going_job_payload =
            JobPayload::new("MerchantGoing", serde_json::to_value(&going_payload)?);
        let going_job = Job::new(
            command.player_id,
            command.village_id as i32,
            travel_time_secs,
            going_job_payload,
        );
        job_repo.add(&going_job).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parabellum_game::{
        models::{buildings::Building, village::Village},
        test_utils::{
            PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions, player_factory,
            valley_factory, village_factory,
        },
    };
    use parabellum_types::Result;
    use parabellum_types::{
        buildings::BuildingName,
        common::{Player, ResourceGroup},
        map::Position,
        tribe::Tribe,
    };

    use super::*;
    use crate::{
        config::Config,
        cqrs::commands::SendResources,
        jobs::tasks::MerchantGoingTask,
        test_utils::tests::{MockUnitOfWork, set_village_resources},
        uow::UnitOfWork,
    };

    async fn setup_test_villages(
        config: &Arc<Config>,
    ) -> Result<(
        Box<dyn UnitOfWork<'static> + 'static>,
        Player,
        Village, // Sender
        Village, // Target
    )> {
        let mock_uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());

        let player_a = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        let valley = valley_factory(ValleyFactoryOptions {
            position: Some(Position { x: 20, y: 20 }),
            ..Default::default()
        });

        let mut village_a = village_factory(VillageFactoryOptions {
            player: Some(player_a.clone()),
            valley: Some(valley),
            ..Default::default()
        });

        let granary =
            Building::new(BuildingName::Granary, config.speed).at_level(10, config.speed)?;
        village_a.add_building_at_slot(granary, 23)?;

        let warehouse =
            Building::new(BuildingName::Warehouse, config.speed).at_level(10, config.speed)?;
        village_a.add_building_at_slot(warehouse, 24)?;

        let marketplace =
            Building::new(BuildingName::Marketplace, config.speed).at_level(3, config.speed)?;
        village_a.add_building_at_slot(marketplace, 25)?;

        set_village_resources(&mut village_a, ResourceGroup(5000, 5000, 5000, 5000));
        let player_b = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        let valley_b = valley_factory(Default::default());
        let mut village_b = village_factory(VillageFactoryOptions {
            player: Some(player_b.clone()),
            valley: Some(valley_b),
            ..Default::default()
        });
        set_village_resources(&mut village_b, ResourceGroup::default());

        mock_uow.players().save(&player_a).await.unwrap();
        mock_uow.players().save(&player_b).await.unwrap();
        mock_uow.villages().save(&village_a).await.unwrap();
        mock_uow.villages().save(&village_b).await.unwrap();

        Ok((mock_uow, player_a, village_a, village_b))
    }

    #[tokio::test]
    async fn test_send_resources_success() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, player_a, village_a, village_b) = setup_test_villages(&config).await?;

        let handler = SendResourcesCommandHandler::new();
        let resources_to_send = ResourceGroup(1000, 500, 0, 0);
        let command = SendResources {
            player_id: player_a.id,
            village_id: village_a.id,
            target_village_id: village_b.id,
            resources: resources_to_send.clone(),
        };

        handler.handle(command, &mock_uow, &config).await?;

        let saved_village_a = mock_uow.villages().get_by_id(village_a.id).await?;
        assert_eq!(saved_village_a.stored_resources().lumber(), 5000 - 1000);
        assert_eq!(saved_village_a.stored_resources().clay(), 5000 - 500);

        let jobs = mock_uow.jobs().list_by_player_id(player_a.id).await?;
        assert_eq!(jobs.len(), 1, "Only 'MerchantGoing' should be created");

        let job = &jobs[0];
        assert_eq!(job.task.task_type, "MerchantGoing");

        let payload: MerchantGoingTask = serde_json::from_value(job.task.data.clone())?;
        let expected_merchants = 2;
        assert_eq!(payload.merchants_used, expected_merchants);
        assert_eq!(payload.destination_village_id, village_b.id);
        assert_eq!(payload.resources.0, resources_to_send.0);
        assert_eq!(payload.resources.1, resources_to_send.1);
        assert!(payload.travel_time_secs > 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_send_resources_fail_no_marketplace() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, player_a, mut village_a, village_b) = setup_test_villages(&config).await?;

        village_a.remove_building_at_slot(25, config.speed)?;
        mock_uow.villages().save(&village_a).await?;

        let handler = SendResourcesCommandHandler::new();
        let command = SendResources {
            player_id: player_a.id,
            village_id: village_a.id,
            target_village_id: village_b.id,
            resources: ResourceGroup(100, 0, 0, 0),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::BuildingRequirementsNotMet {
                building: BuildingName::Marketplace,
                level: 1,
            }
            .to_string()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_send_resources_fail_not_enough_merchants() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, player_a, village_a, village_b) = setup_test_villages(&config).await?;

        let resources_to_send = ResourceGroup(5000, 0, 0, 0); // 11 merchants
        let handler = SendResourcesCommandHandler::new();
        let command = SendResources {
            player_id: player_a.id,
            village_id: village_a.id,
            target_village_id: village_b.id,
            resources: resources_to_send,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::NotEnoughMerchants.to_string()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_send_resources_fail_not_enough_resources() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, player_a, village_a, village_b) = setup_test_villages(&config).await?;
        let resources_to_send = ResourceGroup(5001, 0, 0, 0);

        let handler = SendResourcesCommandHandler::new();
        let command = SendResources {
            player_id: player_a.id,
            village_id: village_a.id,
            target_village_id: village_b.id,
            resources: resources_to_send,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::NotEnoughResources.to_string()
        );
        Ok(())
    }
}
