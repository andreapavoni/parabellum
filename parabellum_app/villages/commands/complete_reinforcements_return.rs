use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::army::Army;
use parabellum_types::errors::AppError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
pub struct CompleteReinforcementsReturn {
    pub action_id: Uuid,
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub home_village_id: u32,
    pub stationed_village_id: u32,
    pub army: Army,
    pub returns_at: DateTime<Utc>,
}

impl Command for CompleteReinforcementsReturn {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.home_village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.home_village_id,
                actual: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![VillageEvent::ReinforcementsReturned {
            action_id: self.action_id,
            movement_id: self.movement_id,
            army_id: self.army_id,
            player_id: self.player_id,
            home_village_id: self.home_village_id,
            stationed_village_id: self.stationed_village_id,
            army: self.army.clone(),
            returns_at: self.returns_at,
        }])
    }
}
