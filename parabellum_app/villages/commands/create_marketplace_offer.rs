use chrono::Utc;
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::village::VillageStocks;
use parabellum_types::{common::ResourceQuantity, errors::GameError};
use uuid::Uuid;

use crate::villages::{
    MarketplaceOfferCreation, VillageAggregate, VillageEvent, commands::as_domain_error,
};

#[derive(Debug, Clone)]
/// Creates a marketplace offer and reserves resources/merchants on the owner village.
pub struct CreateMarketplaceOffer {
    pub player_id: Uuid,
    pub offer_resources: ResourceQuantity,
    pub seek_resources: ResourceQuantity,
    pub speed: i8,
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
        MarketplaceOfferCreation {
            offer_resources: self.offer_resources,
            seek_resources: self.seek_resources,
        }
        .validate()
        .map_err(as_domain_error)?;

        let offer_group: parabellum_types::common::ResourceGroup = self.offer_resources.into();
        let merchants_reserved = aggregate
            .village()
            .schedule_send_resources(offer_group.clone(), self.speed)
            .map_err(as_domain_error)?;

        let now = Utc::now();
        let reservation = owner_reservation_after_create(
            aggregate.village().village.clone(),
            &offer_group,
            merchants_reserved,
        )
        .map_err(as_domain_error)?;

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
                owner_stocks: reservation.stocks,
                owner_busy_merchants: reservation.busy_merchants,
                applied_at: now,
            },
        ])
    }
}

struct MarketplaceReservationState {
    stocks: VillageStocks,
    busy_merchants: u8,
}

fn owner_reservation_after_create(
    mut owner_village: parabellum_game::models::village::Village,
    resources: &parabellum_types::common::ResourceGroup,
    merchants_reserved: u8,
) -> Result<MarketplaceReservationState, GameError> {
    owner_village.reserve_merchant_transfer(resources, merchants_reserved)?;
    Ok(MarketplaceReservationState {
        stocks: owner_village.stocks().clone(),
        busy_merchants: owner_village.busy_merchants,
    })
}
