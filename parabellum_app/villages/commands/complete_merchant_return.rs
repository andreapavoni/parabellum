use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::errors::AppError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
/// Completes merchant return to source village after a delivery trip.
pub struct CompleteMerchantsReturn {
    pub action_id: Uuid,
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub merchants_used: u8,
    pub returns_at: DateTime<Utc>,
}

impl Command for CompleteMerchantsReturn {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.source_village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.source_village_id,
                actual: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![VillageEvent::MerchantsReturned {
            action_id: self.action_id,
            player_id: self.player_id,
            source_village_id: self.source_village_id,
            merchants_used: self.merchants_used,
            returns_at: self.returns_at,
        }])
    }
}
