use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::{common::ResourceGroup, errors::AppError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
/// Completes resource delivery when merchants arrive at target village.
pub struct CompleteMerchantsArrival {
    pub action_id: Uuid,
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub resources: ResourceGroup,
    pub merchants_used: u8,
    pub arrives_at: DateTime<Utc>,
}

impl Command for CompleteMerchantsArrival {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.source_village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.source_village_id,
                actual: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![VillageEvent::MerchantsArrived {
            action_id: self.action_id,
            player_id: self.player_id,
            source_village_id: self.source_village_id,
            target_village_id: self.target_village_id,
            resources: self.resources.clone(),
            merchants_used: self.merchants_used,
            arrives_at: self.arrives_at,
        }])
    }
}
