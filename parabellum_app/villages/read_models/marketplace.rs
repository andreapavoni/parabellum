//! Marketplace read models.
//!
//! Marketplace views aggregate offer and merchant movement state for the
//! application/UI without exposing projection repository details.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use parabellum_game::models::marketplace::MarketplaceOffer;
use parabellum_types::common::ResourceGroup;
use uuid::Uuid;

use crate::read_models::VillageReference;

/// Directional phase of a merchant movement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MerchantMovementKind {
    Going,
    Return,
}

/// App-facing merchant movement summary.
#[derive(Debug, Clone, PartialEq)]
pub struct MerchantMovement {
    pub job_id: Uuid,
    pub kind: MerchantMovementKind,
    pub origin_village_id: u32,
    pub destination_village_id: u32,
    pub resources: ResourceGroup,
    pub merchants_used: u8,
    pub arrives_at: DateTime<Utc>,
}

/// Full marketplace view for a village.
#[derive(Debug, Clone, PartialEq)]
pub struct MarketplaceData {
    pub own_offers: Vec<MarketplaceOffer>,
    pub global_offers: Vec<MarketplaceOffer>,
    pub outgoing_merchants: Vec<MerchantMovement>,
    pub incoming_merchants: Vec<MerchantMovement>,
    pub village_references: HashMap<u32, VillageReference>,
}
