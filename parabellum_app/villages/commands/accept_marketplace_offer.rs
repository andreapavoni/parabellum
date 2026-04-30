use chrono::Utc;
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::villages::{
    VillageAggregate, VillageEvent, commands::as_domain_error, models::MarketplaceOfferSnapshot,
};

#[derive(Debug, Clone)]
/// Accepts an open marketplace offer.
///
/// Owner-side resources/merchants must already be reserved on offer creation; this command only
/// validates and reserves the accepting village side.
pub struct AcceptMarketplaceOffer {
    pub player_id: Uuid,
    pub offer: MarketplaceOfferSnapshot,
    pub owner_arrives_at: chrono::DateTime<chrono::Utc>,
    pub accepting_arrives_at: chrono::DateTime<chrono::Utc>,
}

impl Command for AcceptMarketplaceOffer {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        let accepting_village_id = aggregate.aggregate_id();
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: accepting_village_id,
                player_id: self.player_id,
            }));
        }
        if accepting_village_id == self.offer.owner_village_id
            || self.player_id == self.offer.owner_player_id
        {
            return Err(as_domain_error(GameError::InvalidMarketplaceOffer));
        }
        if self.offer.offer_resources.quantity == 0
            || self.offer.seek_resources.quantity == 0
            || self.offer.offer_resources.resource == self.offer.seek_resources.resource
        {
            return Err(as_domain_error(GameError::InvalidMarketplaceOffer));
        }

        let seek_group: parabellum_types::common::ResourceGroup = self.offer.seek_resources.into();
        let accepting_merchants_used = aggregate
            .village()
            .schedule_send_resources(seek_group)
            .map_err(as_domain_error)?;

        let accepted_at = Utc::now();
        let owner_trip_duration =
            (self.owner_arrives_at - accepted_at).max(chrono::Duration::seconds(1));
        let accepting_trip_duration =
            (self.accepting_arrives_at - accepted_at).max(chrono::Duration::seconds(1));

        Ok(vec![
            VillageEvent::MarketplaceOfferAccepted {
                offer_id: self.offer.offer_id,
                owner_player_id: self.offer.owner_player_id,
                owner_village_id: self.offer.owner_village_id,
                accepting_player_id: self.player_id,
                accepting_village_id,
                offer_resources: self.offer.offer_resources,
                seek_resources: self.offer.seek_resources,
                owner_merchants_reserved: self.offer.merchants_reserved,
                accepting_merchants_used,
                accepted_at,
            },
            VillageEvent::MerchantsTripScheduled {
                arrival_action_id: Uuid::new_v4(),
                return_action_id: Uuid::new_v4(),
                player_id: self.offer.owner_player_id,
                source_village_id: self.offer.owner_village_id,
                target_village_id: accepting_village_id,
                resources: self.offer.offer_resources.into(),
                merchants_used: self.offer.merchants_reserved,
                resources_already_reserved: true,
                arrives_at: self.owner_arrives_at,
                returns_at: self.owner_arrives_at + owner_trip_duration,
            },
            VillageEvent::MerchantsTripScheduled {
                arrival_action_id: Uuid::new_v4(),
                return_action_id: Uuid::new_v4(),
                player_id: self.player_id,
                source_village_id: accepting_village_id,
                target_village_id: self.offer.owner_village_id,
                resources: self.offer.seek_resources.into(),
                merchants_used: accepting_merchants_used,
                resources_already_reserved: false,
                arrives_at: self.accepting_arrives_at,
                returns_at: self.accepting_arrives_at + accepting_trip_duration,
            },
        ])
    }
}
