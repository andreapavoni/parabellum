use crate::{
    command_handlers::helpers::calculate_merchants_needed,
    config::Config,
    cqrs::{CommandHandler, commands::CreateMarketplaceOffer},
    uow::UnitOfWork,
};
use parabellum_game::models::marketplace::MarketplaceOffer;
use parabellum_types::{
    Result,
    buildings::BuildingName,
    errors::{ApplicationError, GameError},
};
use std::sync::Arc;

pub struct CreateMarketplaceOfferCommandHandler {}

impl Default for CreateMarketplaceOfferCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CreateMarketplaceOfferCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<CreateMarketplaceOffer> for CreateMarketplaceOfferCommandHandler {
    async fn handle(
        &self,
        command: CreateMarketplaceOffer,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<()> {
        let village_repo = uow.villages();
        let marketplace_repo = uow.marketplace();

        let mut village = village_repo.get_by_id(command.village_id).await?;

        // Validate marketplace exists
        if village
            .get_building_by_name(&BuildingName::Marketplace)
            .is_none()
        {
            return Err(ApplicationError::Game(
                GameError::BuildingRequirementsNotMet {
                    building: BuildingName::Marketplace,
                    level: 1,
                },
            ));
        }

        // Validate resources
        let offer_resources = command.offer_resources;
        let seek_resources = command.seek_resources;

        if offer_resources.total() == 0 || seek_resources.total() == 0 {
            return Err(ApplicationError::Game(GameError::InvalidMarketplaceOffer));
        }

        // Check if village has enough resources to offer
        village.deduct_resources(&offer_resources)?;

        // Calculate merchants needed
        let merchants_needed = calculate_merchants_needed(&village.tribe, offer_resources.total())?;

        // Check if enough merchants available
        if village.available_merchants() < merchants_needed {
            return Err(ApplicationError::Game(GameError::NotEnoughMerchants));
        }

        // Create marketplace offer
        let offer = MarketplaceOffer::new(
            village.player_id,
            village.id,
            offer_resources,
            seek_resources,
            merchants_needed,
        );

        // Save offer and updated village
        marketplace_repo.create(&offer).await?;
        village_repo.save(&village).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        test_utils::tests::{MockUnitOfWork, set_village_resources},
        uow::UnitOfWork,
    };
    use parabellum_game::{
        models::{buildings::Building, village::Village},
        test_utils::{
            PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions, player_factory,
            valley_factory, village_factory,
        },
    };
    use parabellum_types::{
        common::{Player, ResourceGroup},
        tribe::Tribe,
    };

    async fn setup_test_village(
        config: &Arc<Config>,
        tribe: Tribe,
    ) -> Result<(Box<dyn UnitOfWork<'static> + 'static>, Player, Village)> {
        let mock_uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(tribe),
            ..Default::default()
        });

        let valley = valley_factory(ValleyFactoryOptions::default());
        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            valley: Some(valley),
            ..Default::default()
        });

        // Add marketplace
        let marketplace =
            Building::new(BuildingName::Marketplace, config.speed).at_level(5, config.speed)?;
        village.add_building_at_slot(marketplace, 25)?;

        // Add storage
        let warehouse =
            Building::new(BuildingName::Warehouse, config.speed).at_level(10, config.speed)?;
        village.add_building_at_slot(warehouse, 24)?;

        let granary =
            Building::new(BuildingName::Granary, config.speed).at_level(10, config.speed)?;
        village.add_building_at_slot(granary, 23)?;

        set_village_resources(&mut village, ResourceGroup(5000, 5000, 5000, 5000));

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.villages().save(&village).await.unwrap();

        Ok((mock_uow, player, village))
    }

    #[tokio::test]
    async fn test_create_offer_success() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, _player, village) = setup_test_village(&config, Tribe::Gaul).await?;

        let handler = CreateMarketplaceOfferCommandHandler::new();
        let command = CreateMarketplaceOffer {
            village_id: village.id,
            offer_resources: ResourceGroup(1000, 500, 0, 0), // 1500 total, needs 2 merchants (Gaul: 750 capacity)
            seek_resources: ResourceGroup(0, 0, 1000, 0),
        };

        handler.handle(command, &mock_uow, &config).await?;

        // Verify village resources were deducted
        let updated_village = mock_uow.villages().get_by_id(village.id).await?;
        assert_eq!(updated_village.stored_resources().lumber(), 5000 - 1000);
        assert_eq!(updated_village.stored_resources().clay(), 5000 - 500);

        // Verify offer was created
        let offers = mock_uow.marketplace().list_by_village(village.id).await?;
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].offer_resources.lumber(), 1000);
        assert_eq!(offers[0].seek_resources.iron(), 1000);
        assert_eq!(offers[0].merchants_required, 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_offer_no_marketplace() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, _player, mut village) = setup_test_village(&config, Tribe::Gaul).await?;

        // Remove marketplace
        village.remove_building_at_slot(25, config.speed)?;
        mock_uow.villages().save(&village).await?;

        let handler = CreateMarketplaceOfferCommandHandler::new();
        let command = CreateMarketplaceOffer {
            village_id: village.id,
            offer_resources: ResourceGroup(1000, 0, 0, 0),
            seek_resources: ResourceGroup(0, 1000, 0, 0),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::Game(GameError::BuildingRequirementsNotMet { .. })
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_offer_insufficient_resources() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, _player, village) = setup_test_village(&config, Tribe::Gaul).await?;

        let handler = CreateMarketplaceOfferCommandHandler::new();
        let command = CreateMarketplaceOffer {
            village_id: village.id,
            offer_resources: ResourceGroup(6000, 0, 0, 0), // More than available
            seek_resources: ResourceGroup(0, 1000, 0, 0),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::Game(GameError::NotEnoughResources)
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_offer_zero_resources() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, _player, village) = setup_test_village(&config, Tribe::Gaul).await?;

        let handler = CreateMarketplaceOfferCommandHandler::new();
        let command = CreateMarketplaceOffer {
            village_id: village.id,
            offer_resources: ResourceGroup(0, 0, 0, 0),
            seek_resources: ResourceGroup(0, 1000, 0, 0),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::Game(GameError::InvalidMarketplaceOffer)
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_offer_insufficient_merchants() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, _player, mut village) = setup_test_village(&config, Tribe::Gaul).await?;

        // Downgrade marketplace to level 1 (only 1 merchant)
        village.set_building_level_at_slot(25, 1, config.speed)?;
        mock_uow.villages().save(&village).await?;

        let handler = CreateMarketplaceOfferCommandHandler::new();
        let command = CreateMarketplaceOffer {
            village_id: village.id,
            offer_resources: ResourceGroup(2000, 0, 0, 0), // Needs 3 merchants (Gaul: 750 capacity)
            seek_resources: ResourceGroup(0, 1000, 0, 0),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::Game(GameError::NotEnoughMerchants)
        ));

        Ok(())
    }
}
