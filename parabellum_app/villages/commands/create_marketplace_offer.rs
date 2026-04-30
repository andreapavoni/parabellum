use chrono::Utc;
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::{common::ResourceQuantity, errors::GameError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
/// Creates a marketplace offer and reserves resources/merchants on the owner village.
pub struct CreateMarketplaceOffer {
    pub player_id: Uuid,
    pub offer_resources: ResourceQuantity,
    pub seek_resources: ResourceQuantity,
}

impl Command for CreateMarketplaceOffer {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        let village_id = aggregate.aggregate_id();
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id,
                player_id: self.player_id,
            }));
        }
        if self.offer_resources.quantity == 0
            || self.seek_resources.quantity == 0
            || self.offer_resources.resource == self.seek_resources.resource
        {
            return Err(as_domain_error(GameError::InvalidMarketplaceOffer));
        }
        let (max_side, min_side) = if self.offer_resources.quantity >= self.seek_resources.quantity
        {
            (self.offer_resources.quantity, self.seek_resources.quantity)
        } else {
            (self.seek_resources.quantity, self.offer_resources.quantity)
        };
        if min_side == 0 || max_side > min_side.saturating_mul(3) {
            return Err(as_domain_error(GameError::InvalidMarketplaceOffer));
        }

        let offer_group: parabellum_types::common::ResourceGroup = self.offer_resources.into();
        let merchants_reserved = aggregate
            .village()
            .schedule_send_resources(offer_group)
            .map_err(as_domain_error)?;

        Ok(vec![VillageEvent::MarketplaceOfferCreated {
            offer_id: Uuid::new_v4(),
            owner_player_id: self.player_id,
            owner_village_id: village_id,
            offer_resources: self.offer_resources,
            seek_resources: self.seek_resources,
            merchants_reserved,
            created_at: Utc::now(),
        }])
    }
}
