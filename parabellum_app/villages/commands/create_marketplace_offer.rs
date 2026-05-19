use chrono::Utc;
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::village::VillageStocks;
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
            .schedule_send_resources(offer_group.clone())
            .map_err(as_domain_error)?;

        let now = Utc::now();
        let current = aggregate.village().village.stocks();
        let owner_stocks = VillageStocks {
            warehouse_capacity: current.warehouse_capacity,
            granary_capacity: current.granary_capacity,
            lumber: current.lumber.saturating_sub(offer_group.lumber()),
            clay: current.clay.saturating_sub(offer_group.clay()),
            iron: current.iron.saturating_sub(offer_group.iron()),
            crop: current.crop.saturating_sub(offer_group.crop() as i64),
        };
        let owner_busy_merchants = aggregate
            .village()
            .village
            .busy_merchants
            .saturating_add(merchants_reserved);

        let offer_id = Uuid::new_v4();
        Ok(vec![
            VillageEvent::MarketplaceOfferCreated {
                offer_id,
                owner_player_id: self.player_id,
                owner_village_id: village_id,
                offer_resources: self.offer_resources,
                seek_resources: self.seek_resources,
                merchants_reserved,
                created_at: now,
            },
            VillageEvent::MarketplaceOfferReservationAppliedToVillage {
                offer_id,
                owner_player_id: self.player_id,
                owner_village_id: village_id,
                merchants_reserved,
                owner_stocks,
                owner_busy_merchants,
                applied_at: now,
            },
        ])
    }
}
