use parabellum_core::{ApplicationError, Result};
use parabellum_game::models::marketplace::MarketplaceOffer;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait MarketplaceRepository: Send + Sync {
    /// Creates an offer on db
    async fn create(&self, offer: &MarketplaceOffer) -> Result<(), ApplicationError>;

    /// Gets an offer by id.
    async fn get_by_id(&self, offer_id: Uuid) -> Result<MarketplaceOffer, ApplicationError>;

    /// Lists all offers from a village.
    async fn list_by_village(
        &self,
        village_id: u32,
    ) -> Result<Vec<MarketplaceOffer>, ApplicationError>;

    /// Removes an offer.
    async fn delete(&self, offer_id: Uuid) -> Result<(), ApplicationError>;

    /// Lists all offers (global market).
    async fn list_all(&self) -> Result<Vec<MarketplaceOffer>, ApplicationError>;
}
