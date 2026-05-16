use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::errors::AppError;
use parabellum_types::map::Position;
use parabellum_types::tribe::Tribe;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
pub struct CompleteSettlersArrival {
    pub action_id: Uuid,
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub target_position: Position,
    pub village_name: String,
    pub tribe: Tribe,
    pub arrives_at: DateTime<Utc>,
}

impl Command for CompleteSettlersArrival {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.source_village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.source_village_id,
                actual: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![VillageEvent::SettlersArrived {
            action_id: self.action_id,
            movement_id: self.movement_id,
            army_id: self.army_id,
            player_id: self.player_id,
            source_village_id: self.source_village_id,
            target_village_id: self.target_village_id,
            target_position: self.target_position.clone(),
            village_name: self.village_name.clone(),
            tribe: self.tribe.clone(),
            arrives_at: self.arrives_at,
        }])
    }
}
