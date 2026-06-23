//! Marketplace use-case inputs.
//!
//! These request types describe player intent for merchant transfers and
//! marketplace offers. Use cases load current app context and translate them
//! into command/workflow intent.

use parabellum_types::common::{ResourceGroup, ResourceQuantity};
use uuid::Uuid;

/// Request to load one marketplace offer.
#[derive(Debug, Clone, Copy)]
pub struct GetMarketplaceOfferRequest {
    /// Offer id to load.
    pub offer_id: Uuid,
}

/// Request to load the marketplace view for one village.
#[derive(Debug, Clone, Copy)]
pub struct GetMarketplaceDataRequest {
    /// Village whose marketplace view should be loaded.
    pub village_id: u32,
}

/// Player request to send resources directly from one owned village to another village.
#[derive(Debug, Clone)]
pub struct SendResourcesRequest {
    /// Player expected to own the source village.
    pub player_id: Uuid,
    /// Village that provides merchants and resources.
    pub source_village_id: u32,
    /// Destination village field id.
    pub target_village_id: u32,
    /// Resource payload to transfer.
    pub resources: ResourceGroup,
}

/// Player request to create a marketplace exchange offer.
#[derive(Debug, Clone)]
pub struct CreateMarketplaceOfferRequest {
    /// Player expected to own the village creating the offer.
    pub player_id: Uuid,
    /// Village reserving merchants and offered resources.
    pub village_id: u32,
    /// Resources offered by the owner village.
    pub offer_resources: ResourceQuantity,
    /// Resources requested from the accepting village.
    pub seek_resources: ResourceQuantity,
}

/// Player request to accept an open marketplace offer.
#[derive(Debug, Clone)]
pub struct AcceptMarketplaceOfferRequest {
    /// Player expected to own the accepting village.
    pub player_id: Uuid,
    /// Village accepting the offer and sending requested resources.
    pub village_id: u32,
    /// Offer to claim.
    pub offer_id: Uuid,
}

/// Player request to cancel one of their open marketplace offers.
#[derive(Debug, Clone)]
pub struct CancelMarketplaceOfferRequest {
    /// Player expected to own the offer and village.
    pub player_id: Uuid,
    /// Owner village that created the offer.
    pub village_id: u32,
    /// Offer to cancel.
    pub offer_id: Uuid,
}
