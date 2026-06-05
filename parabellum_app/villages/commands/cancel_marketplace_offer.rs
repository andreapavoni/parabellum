use chrono::Utc;
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::village::{Village, VillageStocks};
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
        let release = owner_reservation_after_cancel(
            aggregate.village().village.clone(),
            &refund,
            self.offer.merchants_reserved,
        );

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
                owner_stocks: release.stocks,
                owner_busy_merchants: release.busy_merchants,
                released_at: now,
            },
        ])
    }
}

struct MarketplaceReservationState {
    stocks: VillageStocks,
    busy_merchants: u8,
}

fn owner_reservation_after_cancel(
    mut owner_village: Village,
    refund: &parabellum_types::common::ResourceGroup,
    merchants_reserved: u8,
) -> MarketplaceReservationState {
    owner_village.release_merchant_transfer(refund, merchants_reserved);
    MarketplaceReservationState {
        stocks: owner_village.stocks().clone(),
        busy_merchants: owner_village.busy_merchants,
    }
}
