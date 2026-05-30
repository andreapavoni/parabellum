use chrono::Utc;
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::village::VillageStocks;
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::villages::{
    VillageAggregate, VillageEvent, commands::as_domain_error, models::MarketplaceOfferSnapshot,
};

#[derive(Debug, Clone)]
/// Cancels an open marketplace offer and releases previously reserved stock/merchants.
pub struct CancelMarketplaceOffer {
    pub player_id: Uuid,
    pub offer: MarketplaceOfferSnapshot,
}

impl Command for CancelMarketplaceOffer {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        let village_id = aggregate.aggregate_id();
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id,
                player_id: self.player_id,
            }));
        }

        let now = Utc::now();
        let refund: parabellum_types::common::ResourceGroup = self.offer.offer_resources.into();
        let current = aggregate.village().village.stocks();
        let owner_stocks = VillageStocks {
            warehouse_capacity: current.warehouse_capacity,
            granary_capacity: current.granary_capacity,
            lumber: current.lumber.saturating_add(refund.lumber()),
            clay: current.clay.saturating_add(refund.clay()),
            iron: current.iron.saturating_add(refund.iron()),
            crop: current.crop.saturating_add(refund.crop() as i64),
        };
        let owner_busy_merchants = aggregate
            .village()
            .village
            .busy_merchants
            .saturating_sub(self.offer.merchants_reserved);

        Ok(vec![
            VillageEvent::MarketplaceOfferCanceled {
                offer_id: self.offer.offer_id,
                owner_player_id: self.player_id,
                owner_village_id: village_id,
                offer_resources: self.offer.offer_resources,
                merchants_reserved: self.offer.merchants_reserved,
                canceled_at: now,
            },
            VillageEvent::MarketplaceOfferReservationReleasedFromVillage {
                offer_id: self.offer.offer_id,
                owner_player_id: self.player_id,
                owner_village_id: village_id,
                merchants_reserved: self.offer.merchants_reserved,
                owner_stocks,
                owner_busy_merchants,
                released_at: now,
            },
        ])
    }
}
