use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::CancelMarketplaceOffer},
    uow::UnitOfWork,
};
use parabellum_types::{
    Result,
    errors::{ApplicationError, GameError},
};
use std::sync::Arc;

pub struct CancelMarketplaceOfferCommandHandler {}

impl Default for CancelMarketplaceOfferCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CancelMarketplaceOfferCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<CancelMarketplaceOffer> for CancelMarketplaceOfferCommandHandler {
    async fn handle(
        &self,
        command: CancelMarketplaceOffer,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<()> {
        let marketplace_repo = uow.marketplace();
        let village_repo = uow.villages();

        // Load offer
        let offer = marketplace_repo.get_by_id(command.offer_id).await?;

        // Validate player owns the offer
        if offer.player_id != command.player_id {
            return Err(ApplicationError::Game(GameError::InvalidMarketplaceOffer));
        }

        // Load village
        let mut village = village_repo.get_by_id(command.village_id).await?;

        // Return resources to village
        village.store_resources(&offer.offer_resources);

        // Delete offer
        marketplace_repo.delete(command.offer_id).await?;

        // Save village
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
    use parabellum_game::models::marketplace::MarketplaceOffer;
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
        tribe::Tribe,
    };

    async fn setup_test_village_with_offer(
        config: &Arc<Config>,
    ) -> Result<(
        Box<dyn UnitOfWork<'static> + 'static>,
        Player,
        Village,
        MarketplaceOffer,
    )> {
        let mock_uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        let valley = valley_factory(ValleyFactoryOptions::default());
        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            valley: Some(valley),
            ..Default::default()
        });

        let marketplace =
            Building::new(BuildingName::Marketplace, config.speed).at_level(5, config.speed)?;
        village.add_building_at_slot(marketplace, 25)?;

        let warehouse =
            Building::new(BuildingName::Warehouse, config.speed).at_level(15, config.speed)?;
        village.add_building_at_slot(warehouse, 26)?;

        let granary =
            Building::new(BuildingName::Granary, config.speed).at_level(15, config.speed)?;
        village.add_building_at_slot(granary, 27)?;

        set_village_resources(&mut village, ResourceGroup(3000, 3000, 3000, 3000));

        // Create offer (resources would have been deducted)
        let offer = MarketplaceOffer::new(
            player.id,
            village.id,
            ResourceGroup(1000, 500, 0, 0),
            ResourceGroup(0, 0, 1000, 0),
            2,
        );

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.villages().save(&village).await.unwrap();
        mock_uow.marketplace().create(&offer).await.unwrap();

        Ok((mock_uow, player, village, offer))
    }

    #[tokio::test]
    async fn test_cancel_offer_success() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, player, village, offer) = setup_test_village_with_offer(&config).await?;

        let handler = CancelMarketplaceOfferCommandHandler::new();
        let command = CancelMarketplaceOffer {
            player_id: player.id,
            village_id: village.id,
            offer_id: offer.id,
        };

        handler.handle(command, &mock_uow, &config).await?;

        // Verify resources were returned
        let updated_village = mock_uow.villages().get_by_id(village.id).await?;
        assert_eq!(updated_village.stored_resources().lumber(), 3000 + 1000);
        assert_eq!(updated_village.stored_resources().clay(), 3000 + 500);

        // Verify offer was deleted
        let offers = mock_uow.marketplace().list_by_village(village.id).await?;
        assert_eq!(offers.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_cancel_offer_not_owner() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, _player, village, offer) = setup_test_village_with_offer(&config).await?;

        // Different player trying to cancel
        let other_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        mock_uow.players().save(&other_player).await?;

        let handler = CancelMarketplaceOfferCommandHandler::new();
        let command = CancelMarketplaceOffer {
            player_id: other_player.id, // Wrong player!
            village_id: village.id,
            offer_id: offer.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::Game(GameError::InvalidMarketplaceOffer)
        ));

        Ok(())
    }
}
