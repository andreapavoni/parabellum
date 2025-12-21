use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::AcceptMarketplaceOffer},
    jobs::{Job, JobPayload, tasks::MerchantGoingTask},
    uow::UnitOfWork,
};
use parabellum_types::{
    Result,
    buildings::BuildingName,
    errors::{ApplicationError, GameError},
};
use std::sync::Arc;

pub struct AcceptMarketplaceOfferCommandHandler {}

impl Default for AcceptMarketplaceOfferCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl AcceptMarketplaceOfferCommandHandler {
    pub fn new() -> Self {
        Self {}
    }

    /// Calculates the amount of merchants needed to transport the amount of resources.
    fn calculate_merchants_needed(capacity: u32, resources_total: u32) -> Result<u8, GameError> {
        if capacity == 0 {
            return Err(GameError::NotEnoughMerchants);
        }

        let merchants = (resources_total as f64 / capacity as f64).ceil() as u8;
        if resources_total > 0 && merchants == 0 {
            Ok(1)
        } else {
            Ok(merchants)
        }
    }
}

#[async_trait::async_trait]
impl CommandHandler<AcceptMarketplaceOffer> for AcceptMarketplaceOfferCommandHandler {
    async fn handle(
        &self,
        command: AcceptMarketplaceOffer,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<()> {
        let marketplace_repo = uow.marketplace();
        let village_repo = uow.villages();
        let job_repo = uow.jobs();

        // Load offer
        let offer = marketplace_repo.get_by_id(command.offer_id).await?;

        // Validate acceptor is not the offerer
        if offer.player_id == command.player_id {
            return Err(ApplicationError::Game(GameError::InvalidMarketplaceOffer));
        }

        // Load both villages
        let mut acceptor_village = village_repo.get_by_id(command.village_id).await?;
        let offerer_village = village_repo.get_by_id(offer.village_id).await?;

        // Validate acceptor has marketplace
        if acceptor_village
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

        // Validate offerer still has marketplace (offer still valid)
        if offerer_village
            .get_building_by_name(&BuildingName::Marketplace)
            .is_none()
        {
            return Err(ApplicationError::Game(GameError::OfferNoLongerValid));
        }

        // Calculate merchants needed for acceptor
        let acceptor_merchant_stats = acceptor_village.tribe.merchant_stats();
        let acceptor_merchants_needed = Self::calculate_merchants_needed(
            acceptor_merchant_stats.capacity,
            offer.seek_resources.total(),
        )?;

        // Validate acceptor has enough seek_resources
        acceptor_village.deduct_resources(&offer.seek_resources)?;

        // Validate acceptor has enough available merchants
        if acceptor_village.available_merchants() < acceptor_merchants_needed {
            return Err(ApplicationError::Game(GameError::NotEnoughMerchants));
        }

        // Calculate travel times
        let offerer_merchant_stats = offerer_village.tribe.merchant_stats();
        let offerer_to_acceptor_travel_time = offerer_village.position.calculate_travel_time_secs(
            acceptor_village.position.clone(),
            offerer_merchant_stats.speed,
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        let acceptor_to_offerer_travel_time = acceptor_village.position.calculate_travel_time_secs(
            offerer_village.position,
            acceptor_merchant_stats.speed,
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        // Create merchant jobs: offerer → acceptor (delivering offer_resources)
        // The MerchantGoingJobHandler will automatically create the return job
        let offerer_going_payload = MerchantGoingTask {
            origin_village_id: offerer_village.id,
            destination_village_id: acceptor_village.id,
            resources: offer.offer_resources.clone(),
            merchants_used: offer.merchants_required,
            travel_time_secs: offerer_to_acceptor_travel_time,
        };
        let offerer_going_job = Job::new(
            offer.player_id,
            offerer_village.id as i32,
            offerer_to_acceptor_travel_time,
            JobPayload::new(
                "MerchantGoing",
                serde_json::to_value(&offerer_going_payload)?,
            ),
        );
        job_repo.add(&offerer_going_job).await?;

        // Create merchant jobs: acceptor → offerer (delivering seek_resources)
        // The MerchantGoingJobHandler will automatically create the return job
        let acceptor_going_payload = MerchantGoingTask {
            origin_village_id: acceptor_village.id,
            destination_village_id: offerer_village.id,
            resources: offer.seek_resources.clone(),
            merchants_used: acceptor_merchants_needed,
            travel_time_secs: acceptor_to_offerer_travel_time,
        };
        let acceptor_going_job = Job::new(
            command.player_id,
            acceptor_village.id as i32,
            acceptor_to_offerer_travel_time,
            JobPayload::new(
                "MerchantGoing",
                serde_json::to_value(&acceptor_going_payload)?,
            ),
        );
        job_repo.add(&acceptor_going_job).await?;

        // Delete the offer (trade is now in progress)
        marketplace_repo.delete(command.offer_id).await?;

        // Save acceptor village (resources deducted)
        village_repo.save(&acceptor_village).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use parabellum_game::{
        models::{buildings::Building, marketplace::MarketplaceOffer, village::Village},
        test_utils::setup_player_party,
    };
    use parabellum_types::{
        army::TroopSet,
        common::{Player, ResourceGroup},
        map::Position,
        tribe::Tribe,
    };

    use super::*;
    use crate::{
        config::Config,
        test_utils::tests::{MockUnitOfWork, set_village_resources},
        uow::UnitOfWork,
    };

    async fn setup_test_villages(
        config: &Arc<Config>,
    ) -> Result<(
        Box<dyn UnitOfWork<'static> + 'static>,
        Player,
        Village,
        Player,
        Village,
        MarketplaceOffer,
    )> {
        let mock_uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());

        let warehouse =
            Building::new(BuildingName::Warehouse, config.speed).at_level(10, config.speed)?;
        let granary =
            Building::new(BuildingName::Granary, config.speed).at_level(10, config.speed)?;
        let marketplace =
            Building::new(BuildingName::Marketplace, config.speed).at_level(10, config.speed)?;

        let (offerer_player, mut offerer_village, _, _) = setup_player_party(
            Some(Position { x: 0, y: 0 }),
            Tribe::Gaul,
            TroopSet::default(),
            false,
        )?;

        offerer_village.add_building_at_slot(marketplace.clone(), 25)?;
        offerer_village.add_building_at_slot(warehouse.clone(), 24)?;
        offerer_village.add_building_at_slot(granary.clone(), 23)?;
        set_village_resources(&mut offerer_village, ResourceGroup(1000, 500, 0, 0));

        let (acceptor_player, mut acceptor_village, _, _) = setup_player_party(
            Some(Position { x: 10, y: 10 }),
            Tribe::Roman,
            TroopSet::default(),
            false,
        )?;
        acceptor_village.add_building_at_slot(marketplace, 25)?;
        acceptor_village.add_building_at_slot(warehouse, 24)?;
        acceptor_village.add_building_at_slot(granary, 23)?;
        set_village_resources(&mut acceptor_village, ResourceGroup(0, 0, 5000, 0));

        // Create offer
        let offer = MarketplaceOffer::new(
            offerer_player.id,
            offerer_village.id,
            ResourceGroup(1000, 0, 0, 0), // Offering
            ResourceGroup(0, 0, 1000, 0), // Seeking
            2,                            // merchants_required
        );

        mock_uow.players().save(&offerer_player).await.unwrap();
        mock_uow.players().save(&acceptor_player).await.unwrap();
        mock_uow.villages().save(&offerer_village).await.unwrap();
        mock_uow.villages().save(&acceptor_village).await.unwrap();
        mock_uow.marketplace().create(&offer).await.unwrap();

        Ok((
            mock_uow,
            offerer_player,
            offerer_village,
            acceptor_player,
            acceptor_village,
            offer,
        ))
    }

    #[tokio::test]
    async fn test_accept_offer_success() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, offerer_player, offerer_village, acceptor_player, acceptor_village, offer) =
            setup_test_villages(&config).await?;

        let handler = AcceptMarketplaceOfferCommandHandler::new();
        let command = AcceptMarketplaceOffer {
            player_id: acceptor_player.id,
            village_id: acceptor_village.id,
            offer_id: offer.id,
        };

        handler.handle(command, &mock_uow, &config).await?;

        // Verify acceptor's resources were deducted
        let updated_acceptor = mock_uow.villages().get_by_id(acceptor_village.id).await?;
        assert_eq!(updated_acceptor.stored_resources().iron(), 5000 - 1000);

        // Verify offer was deleted
        let offers = mock_uow
            .marketplace()
            .list_by_village(offerer_village.id)
            .await?;
        assert_eq!(offers.len(), 0);

        // Verify merchant jobs were created (2 MerchantGoing jobs, return jobs created by handlers)
        let offerer_jobs = mock_uow.jobs().list_by_player_id(offerer_player.id).await?;
        let acceptor_jobs = mock_uow
            .jobs()
            .list_by_player_id(acceptor_player.id)
            .await?;
        assert_eq!(offerer_jobs.len(), 1);
        assert_eq!(acceptor_jobs.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_accept_own_offer_fails() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, offerer_player, offerer_village, _acceptor_player, _acceptor_village, offer) =
            setup_test_villages(&config).await?;

        let handler = AcceptMarketplaceOfferCommandHandler::new();
        let command = AcceptMarketplaceOffer {
            player_id: offerer_player.id, // Same player!
            village_id: offerer_village.id,
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

    #[tokio::test]
    async fn test_accept_offer_insufficient_resources() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (
            mock_uow,
            _offerer_player,
            _offerer_village,
            acceptor_player,
            mut acceptor_village,
            offer,
        ) = setup_test_villages(&config).await?;

        // Remove acceptor's resources
        set_village_resources(&mut acceptor_village, ResourceGroup(0, 0, 0, 0));
        mock_uow.villages().save(&acceptor_village).await?;

        let handler = AcceptMarketplaceOfferCommandHandler::new();
        let command = AcceptMarketplaceOffer {
            player_id: acceptor_player.id,
            village_id: acceptor_village.id,
            offer_id: offer.id,
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
    async fn test_accept_offer_no_marketplace() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (
            mock_uow,
            _offerer_player,
            _offerer_village,
            acceptor_player,
            mut acceptor_village,
            offer,
        ) = setup_test_villages(&config).await?;

        // Remove acceptor's marketplace
        acceptor_village.remove_building_at_slot(25, config.speed)?;
        mock_uow.villages().save(&acceptor_village).await?;

        let handler = AcceptMarketplaceOfferCommandHandler::new();
        let command = AcceptMarketplaceOffer {
            player_id: acceptor_player.id,
            village_id: acceptor_village.id,
            offer_id: offer.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::Game(GameError::BuildingRequirementsNotMet { .. })
        ));

        Ok(())
    }
}
