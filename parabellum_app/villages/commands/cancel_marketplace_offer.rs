use chrono::Utc;
use mini_cqrs_es::{Aggregate, Command, CqrsError};
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

        Ok(vec![VillageEvent::MarketplaceOfferCanceled {
            offer_id: self.offer.offer_id,
            owner_player_id: self.player_id,
            owner_village_id: village_id,
            offer_resources: self.offer.offer_resources,
            merchants_reserved: self.offer.merchants_reserved,
            canceled_at: Utc::now(),
        }])
    }
}
