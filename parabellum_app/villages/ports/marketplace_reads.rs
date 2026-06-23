//! Read/context port for marketplace use cases.
//!
//! Marketplace use cases require current village and offer state to plan
//! merchant travel and validate offer ownership/status before delegating
//! command execution to infrastructure.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::models::{MarketplaceOfferModel, VillageModel};
use crate::villages::read_models::MarketplaceData;

/// Loads marketplace read-model context required by app use cases.
#[async_trait]
pub trait MarketplaceReadPort: Send + Sync {
    /// Returns the current village read model used for ownership and travel planning.
    async fn get_marketplace_village(
        &self,
        village_id: u32,
    ) -> Result<VillageModel, ApplicationError>;

    /// Returns the current marketplace offer read model used for validation and claiming.
    async fn get_marketplace_offer(
        &self,
        offer_id: Uuid,
    ) -> Result<MarketplaceOfferModel, ApplicationError>;

    /// Returns the full marketplace view for a village.
    async fn get_marketplace_data(
        &self,
        village_id: u32,
    ) -> Result<MarketplaceData, ApplicationError>;
}
