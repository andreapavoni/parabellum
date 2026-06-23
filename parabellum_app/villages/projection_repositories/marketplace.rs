//! Marketplace projection repository contracts.

use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::{
    models::{MarketplaceOfferModel, MarketplaceOfferStatus},
    read_models::MerchantMovement,
};

/// Filter for marketplace offer projection queries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MarketplaceOfferListFilter {
    pub owner_village_id: Option<u32>,
    pub exclude_owner_village_id: Option<u32>,
    pub status: Option<MarketplaceOfferStatus>,
}

impl MarketplaceOfferListFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn owner_village(mut self, village_id: u32) -> Self {
        self.owner_village_id = Some(village_id);
        self
    }

    pub fn excluding_owner_village(mut self, village_id: u32) -> Self {
        self.exclude_owner_village_id = Some(village_id);
        self
    }

    pub fn status(mut self, status: MarketplaceOfferStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn open(self) -> Self {
        self.status(MarketplaceOfferStatus::Open)
    }
}

/// Persistence boundary for marketplace offer and merchant movement projections.
#[async_trait::async_trait]
pub trait MarketplaceRepository: Send + Sync {
    async fn upsert(&self, offer: &MarketplaceOfferModel) -> Result<(), ApplicationError>;

    async fn get_by_offer_id(
        &self,
        offer_id: Uuid,
    ) -> Result<MarketplaceOfferModel, ApplicationError>;

    async fn set_status(
        &self,
        offer_id: Uuid,
        status: MarketplaceOfferStatus,
        accepted_by_player_id: Option<Uuid>,
        accepted_by_village_id: Option<u32>,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), ApplicationError>;

    async fn list_by_owner_village_id(
        &self,
        village_id: u32,
    ) -> Result<Vec<MarketplaceOfferModel>, ApplicationError> {
        self.list_offers(MarketplaceOfferListFilter::new().owner_village(village_id))
            .await
    }

    async fn list_offers(
        &self,
        filter: MarketplaceOfferListFilter,
    ) -> Result<Vec<MarketplaceOfferModel>, ApplicationError>;

    async fn list_open(&self) -> Result<Vec<MarketplaceOfferModel>, ApplicationError> {
        self.list_offers(MarketplaceOfferListFilter::new().open())
            .await
    }

    async fn claim_open_for_accept(
        &self,
        offer_id: Uuid,
        accepted_by_player_id: Uuid,
        accepted_by_village_id: u32,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<MarketplaceOfferModel>, ApplicationError>;

    async fn list_active_outgoing(
        &self,
        village_id: u32,
    ) -> Result<Vec<MerchantMovement>, ApplicationError>;

    async fn list_active_incoming(
        &self,
        village_id: u32,
    ) -> Result<Vec<MerchantMovement>, ApplicationError>;
}
