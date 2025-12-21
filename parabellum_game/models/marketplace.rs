use chrono::{DateTime, Utc};
use parabellum_types::common::ResourceGroup;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketplaceOffer {
    pub id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub offer_resources: ResourceGroup,
    pub seek_resources: ResourceGroup,
    pub merchants_required: u8,
    pub created_at: DateTime<Utc>,
}

impl MarketplaceOffer {
    pub fn new(
        player_id: Uuid,
        village_id: u32,
        offer_resources: ResourceGroup,
        seek_resources: ResourceGroup,
        merchants_required: u8,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            player_id,
            village_id,
            offer_resources,
            seek_resources,
            merchants_required,
            created_at: Utc::now(),
        }
    }
}
