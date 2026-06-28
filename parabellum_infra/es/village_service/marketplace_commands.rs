//! Marketplace command orchestration for `VillageEsService`.

use mini_cqrs_es::CqrsError;
use mini_cqrs_es::anyhow::Result;
use parabellum_app::villages::models::{MarketplaceOfferSnapshot, MarketplaceOfferStatus};
use parabellum_app::villages::projection_repositories::MarketplaceRepository;
use parabellum_app::villages::{
    CancelMarketplaceOffer, CreateMarketplaceOffer, MarketplaceAcceptance, SendMerchantsTransfer,
    VillageArmyContext, VillageService, hydrate_village,
};
use parabellum_types::errors::GameError;

use crate::es::workflows;
use crate::es::{PostgresMarketplaceRepository, village_cqrs_runtime};

use super::VillageEsService;

impl VillageEsService {
    fn as_offer_snapshot(
        offer: parabellum_app::villages::models::MarketplaceOfferModel,
    ) -> MarketplaceOfferSnapshot {
        MarketplaceOfferSnapshot {
            offer_id: offer.offer_id,
            owner_player_id: offer.owner_player_id,
            owner_village_id: offer.owner_village_id,
            offer_resources: offer.offer_resources,
            seek_resources: offer.seek_resources,
            merchants_reserved: offer.merchants_reserved,
        }
    }

    pub async fn send_resources(
        &self,
        village_id: u32,
        command: &SendMerchantsTransfer,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_resources(village_id, command).await
    }

    pub async fn create_marketplace_offer(
        &self,
        village_id: u32,
        command: &CreateMarketplaceOffer,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.create_marketplace_offer(village_id, command).await
    }

    pub async fn cancel_marketplace_offer(
        &self,
        village_id: u32,
        player_id: uuid::Uuid,
        offer_id: uuid::Uuid,
    ) -> Result<u32, CqrsError> {
        let offer = self.get_marketplace_offer(offer_id).await?;
        if offer.status != MarketplaceOfferStatus::Open
            || offer.owner_village_id != village_id
            || offer.owner_player_id != player_id
        {
            return Err(CqrsError::domain_source(GameError::InvalidMarketplaceOffer));
        }

        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service
            .cancel_marketplace_offer(
                village_id,
                &CancelMarketplaceOffer {
                    player_id,
                    offer: Self::as_offer_snapshot(offer),
                },
            )
            .await
    }

    pub async fn accept_marketplace_offer(
        &self,
        accepting_village_id: u32,
        accepting_player_id: uuid::Uuid,
        offer_id: uuid::Uuid,
        owner_arrives_at: chrono::DateTime<chrono::Utc>,
        accepting_arrives_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(accepting_village_id, accepting_player_id)
            .await?;
        let offers =
            PostgresMarketplaceRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        let offer = offers
            .get_by_offer_id(offer_id)
            .await
            .map_err(CqrsError::domain_source)?;
        if offer.status != MarketplaceOfferStatus::Open {
            return Err(CqrsError::domain_source(
                GameError::MarketplaceOfferNoLongerValid,
            ));
        }
        let offer_snapshot = Self::as_offer_snapshot(offer.clone());
        MarketplaceAcceptance {
            accepting_player_id,
            accepting_village_id,
            offer: &offer_snapshot,
        }
        .validate()
        .map_err(CqrsError::domain_source)?;

        let accepting_model = self.get_village(accepting_village_id).await?;
        if accepting_model.player_id != accepting_player_id {
            return Err(CqrsError::domain_source(GameError::VillageNotOwned {
                village_id: accepting_village_id,
                player_id: accepting_player_id,
            }));
        }
        let accepting_village = hydrate_village(accepting_model, VillageArmyContext::default());
        let seek_group: parabellum_types::common::ResourceGroup = offer.seek_resources.into();
        let accepting_merchants_used = accepting_village
            .validate_merchant_transfer(
                &seek_group,
                parabellum_app::config::Config::from_env().speed,
            )
            .map_err(CqrsError::domain_source)?;
        let mut accepting_after = accepting_village.clone();
        accepting_after
            .reserve_merchant_transfer(&seek_group, accepting_merchants_used)
            .map_err(CqrsError::domain_source)?;

        let accepted_at = chrono::Utc::now();
        let Some(offer) = offers
            .claim_open_for_accept(
                offer_id,
                accepting_player_id,
                accepting_village_id,
                accepted_at,
            )
            .await
            .map_err(CqrsError::domain_source)?
        else {
            return Err(CqrsError::domain_source(
                GameError::MarketplaceOfferNoLongerValid,
            ));
        };
        self.append_workflow_events(workflows::merchants::offer_acceptance_events(
            workflows::merchants::OfferAcceptanceWorkflowInput {
                offer: &offer,
                accepting_player_id,
                accepting_village_id,
                accepting_stocks: accepting_after.stocks().clone(),
                accepting_busy_merchants: accepting_after.busy_merchants,
                accepting_merchants_used,
                accepted_at,
                owner_arrives_at,
                accepting_arrives_at,
            },
        ))
        .await?;
        Ok(accepting_village_id)
    }
}
