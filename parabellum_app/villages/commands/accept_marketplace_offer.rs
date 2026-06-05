use chrono::Utc;
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::villages::{
    MarketplaceAcceptance, VillageAggregate, VillageEvent, commands::as_domain_error,
    models::MarketplaceOfferSnapshot,
};

#[derive(Debug, Clone)]
/// Accepts an open marketplace offer.
///
/// Owner-side resources/merchants must already be reserved on offer creation; this command only
/// validates acceptance and emits the acceptance fact.
///
/// Cross-stream merchant trip scheduling is orchestrated by service-layer workflow append.
pub struct AcceptMarketplaceOffer {
    pub player_id: Uuid,
    pub offer: MarketplaceOfferSnapshot,
    pub owner_arrives_at: chrono::DateTime<chrono::Utc>,
    pub accepting_arrives_at: chrono::DateTime<chrono::Utc>,
    pub speed: i8,
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
        MarketplaceAcceptance {
            accepting_player_id: self.player_id,
            accepting_village_id,
            offer: &self.offer,
        }
        .validate()
        .map_err(as_domain_error)?;

        let seek_group: parabellum_types::common::ResourceGroup = self.offer.seek_resources.into();
        let accepting_merchants_used = aggregate
            .village()
            .schedule_send_resources(seek_group, self.speed)
            .map_err(as_domain_error)?;

        let accepted_at = Utc::now();

        Ok(vec![VillageEvent::MarketplaceOfferAccepted {
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
        }])
    }
}
