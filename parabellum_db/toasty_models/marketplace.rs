use uuid::Uuid;

use crate::toasty_time::{chrono_to_jiff_utc, jiff_to_chrono_utc};
use parabellum_game::models::marketplace::MarketplaceOffer;
use parabellum_types::{common::ResourceGroup, errors::ApplicationError};

#[derive(Debug, Clone, toasty::Model)]
#[table = "marketplace_offers"]
pub struct MarketplaceOfferDbRow {
    #[key]
    pub id: Uuid,
    #[index]
    pub player_id: Uuid,
    #[index]
    pub village_id: i32,

    #[serialize(json)]
    pub offer_resources: ResourceGroup,
    #[serialize(json)]
    pub seek_resources: ResourceGroup,

    pub merchants_required: i16,
    pub created_at: jiff::Timestamp,
}

impl TryFrom<MarketplaceOfferDbRow> for MarketplaceOffer {
    type Error = ApplicationError;

    fn try_from(value: MarketplaceOfferDbRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            player_id: value.player_id,
            village_id: value.village_id as u32,
            offer_resources: value.offer_resources,
            seek_resources: value.seek_resources,
            merchants_required: value.merchants_required as u8,
            created_at: jiff_to_chrono_utc(value.created_at)?,
        })
    }
}

impl TryFrom<&MarketplaceOffer> for MarketplaceOfferDbRow {
    type Error = ApplicationError;

    fn try_from(value: &MarketplaceOffer) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            player_id: value.player_id,
            village_id: value.village_id as i32,
            offer_resources: value.offer_resources.clone(),
            seek_resources: value.seek_resources.clone(),
            merchants_required: value.merchants_required as i16,
            created_at: chrono_to_jiff_utc(value.created_at)?,
        })
    }
}
