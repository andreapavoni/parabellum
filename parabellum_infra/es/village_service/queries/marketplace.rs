//! Marketplace read composition for `VillageEsService`.
//!
//! This module builds marketplace-facing views from offer, merchant movement,
//! and village reference projections. It does not apply marketplace rules; those
//! remain in the app use cases and game domain.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use mini_cqrs_es::{CqrsError, QueryRunner};
use parabellum_app::read_models::VillageReference;
use parabellum_app::villages::cqrs_queries::{GetMarketplaceOfferById, GetOpenMarketplaceOffers};
use parabellum_app::villages::models::MarketplaceOfferModel;
use parabellum_app::villages::projection_repositories::{
    MerchantMovementRepository, VillageRepository,
};
use parabellum_app::villages::read_models::MarketplaceData;
use parabellum_game::models::marketplace::MarketplaceOffer;

use crate::es::{
    PostgresMarketplaceRepository, PostgresMerchantMovementRepository, PostgresVillageRepository,
    village_cqrs_runtime,
};

use super::super::VillageEsService;

impl VillageEsService {
    /// Returns open marketplace offers visible to the caller.
    pub async fn get_open_marketplace_offers(
        &self,
    ) -> Result<Vec<MarketplaceOfferModel>, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&GetOpenMarketplaceOffers {
                repository: Arc::new(PostgresMarketplaceRepository::new(
                    crate::ProjectionDb::new(self.pool.clone()),
                )),
            })
            .await
    }

    /// Returns one marketplace offer by id.
    pub async fn get_marketplace_offer(
        &self,
        offer_id: uuid::Uuid,
    ) -> Result<MarketplaceOfferModel, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&GetMarketplaceOfferById {
                repository: Arc::new(PostgresMarketplaceRepository::new(
                    crate::ProjectionDb::new(self.pool.clone()),
                )),
                offer_id,
            })
            .await
    }

    /// Returns the marketplace page read model for a village.
    pub async fn get_marketplace_data(
        &self,
        village_id: u32,
    ) -> Result<MarketplaceData, CqrsError> {
        let projection_db = crate::ProjectionDb::new(self.pool.clone());
        let marketplace_repo = PostgresMarketplaceRepository::new(projection_db.clone());
        let merchant_movement_repo = PostgresMerchantMovementRepository::new(projection_db);

        let own_open_models = marketplace_repo
            .list_open_by_owner_village_id(village_id)
            .await
            .map_err(CqrsError::domain_source)?;
        let global_open_models = marketplace_repo
            .list_open_excluding_owner_village_id(village_id)
            .await
            .map_err(CqrsError::domain_source)?;
        let merchant_movements = merchant_movement_repo
            .list_active_for_village(village_id)
            .await
            .map_err(CqrsError::domain_source)?;

        let village_ids =
            own_open_models
                .iter()
                .chain(global_open_models.iter())
                .map(|offer| offer.owner_village_id)
                .chain(merchant_movements.iter().flat_map(|movement| {
                    [movement.origin_village_id, movement.destination_village_id]
                }))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
        let village_references = self.marketplace_village_references(village_ids).await?;

        Ok(MarketplaceData {
            own_offers: own_open_models
                .iter()
                .cloned()
                .map(marketplace_offer)
                .collect(),
            global_offers: global_open_models
                .into_iter()
                .map(marketplace_offer)
                .collect(),
            merchant_movements,
            village_references,
        })
    }

    async fn marketplace_village_references(
        &self,
        village_ids: Vec<u32>,
    ) -> Result<HashMap<u32, VillageReference>, CqrsError> {
        if village_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let repo = PostgresVillageRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        let villages = repo
            .list_by_village_ids(&village_ids)
            .await
            .map_err(CqrsError::domain_source)?;

        Ok(villages
            .into_iter()
            .map(|village| {
                (
                    village.village_id,
                    VillageReference {
                        id: village.village_id,
                        name: village.village_name,
                        position: village.position,
                    },
                )
            })
            .collect())
    }
}

fn marketplace_offer(model: MarketplaceOfferModel) -> MarketplaceOffer {
    MarketplaceOffer {
        id: model.offer_id,
        player_id: model.owner_player_id,
        village_id: model.owner_village_id,
        offer_resources: model.offer_resources,
        seek_resources: model.seek_resources,
        merchants_required: model.merchants_reserved,
        created_at: model.created_at,
    }
}
