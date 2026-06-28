//! Typed SQL rows for marketplace projections.

use chrono::{DateTime, Utc};
use parabellum_app::villages::models::{MarketplaceOfferModel, MarketplaceOfferStatus};
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "rm_marketplace_offer_status", rename_all = "snake_case")]
pub(super) enum DbMarketplaceOfferStatus {
    Open,
    Accepted,
    Canceled,
}

impl From<MarketplaceOfferStatus> for DbMarketplaceOfferStatus {
    fn from(value: MarketplaceOfferStatus) -> Self {
        match value {
            MarketplaceOfferStatus::Open => Self::Open,
            MarketplaceOfferStatus::Accepted => Self::Accepted,
            MarketplaceOfferStatus::Canceled => Self::Canceled,
        }
    }
}

impl From<DbMarketplaceOfferStatus> for MarketplaceOfferStatus {
    fn from(value: DbMarketplaceOfferStatus) -> Self {
        match value {
            DbMarketplaceOfferStatus::Open => Self::Open,
            DbMarketplaceOfferStatus::Accepted => Self::Accepted,
            DbMarketplaceOfferStatus::Canceled => Self::Canceled,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub(super) struct DbMarketplaceOfferRow {
    offer_id: Uuid,
    owner_player_id: Uuid,
    owner_village_id: i32,
    offer_resources: Json<parabellum_types::common::ResourceQuantity>,
    seek_resources: Json<parabellum_types::common::ResourceQuantity>,
    merchants_reserved: i16,
    status: DbMarketplaceOfferStatus,
    accepted_by_player_id: Option<Uuid>,
    accepted_by_village_id: Option<i32>,
    created_at: DateTime<Utc>,
    accepted_at: Option<DateTime<Utc>>,
    canceled_at: Option<DateTime<Utc>>,
}

impl From<DbMarketplaceOfferRow> for MarketplaceOfferModel {
    fn from(row: DbMarketplaceOfferRow) -> Self {
        Self {
            offer_id: row.offer_id,
            owner_player_id: row.owner_player_id,
            owner_village_id: row.owner_village_id as u32,
            offer_resources: row.offer_resources.0,
            seek_resources: row.seek_resources.0,
            merchants_reserved: row.merchants_reserved as u8,
            status: row.status.into(),
            accepted_by_player_id: row.accepted_by_player_id,
            accepted_by_village_id: row.accepted_by_village_id.map(|v| v as u32),
            created_at: row.created_at,
            accepted_at: row.accepted_at,
            canceled_at: row.canceled_at,
        }
    }
}
