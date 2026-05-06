use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::errors::AppError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
pub struct ConquerVillage {
    pub player_id: Uuid,
    pub village_id: u32,
}

impl Command for ConquerVillage {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.village_id,
                actual: aggregate.aggregate_id(),
            }));
        }
        Ok(vec![VillageEvent::VillageConquered {
            player_id: self.player_id,
        }])
    }
}
