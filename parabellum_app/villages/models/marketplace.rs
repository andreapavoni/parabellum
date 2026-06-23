//! Marketplace projection models.

use chrono::{DateTime, Utc};
use parabellum_types::common::ResourceQuantity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Projection status for marketplace offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketplaceOfferStatus {
    Open,
    Accepted,
    Canceled,
}

/// Projected marketplace offer row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceOfferModel {
    pub offer_id: Uuid,
    pub owner_player_id: Uuid,
    pub owner_village_id: u32,
    pub offer_resources: ResourceQuantity,
    pub seek_resources: ResourceQuantity,
    pub merchants_reserved: u8,
    pub status: MarketplaceOfferStatus,
    pub accepted_by_player_id: Option<Uuid>,
    pub accepted_by_village_id: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub canceled_at: Option<DateTime<Utc>>,
}

/// Domain snapshot used for marketplace offer command orchestration.
///
/// This is intentionally decoupled from projection-specific read model structs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceOfferSnapshot {
    pub offer_id: Uuid,
    pub owner_player_id: Uuid,
    pub owner_village_id: u32,
    pub offer_resources: ResourceQuantity,
    pub seek_resources: ResourceQuantity,
    pub merchants_reserved: u8,
}
