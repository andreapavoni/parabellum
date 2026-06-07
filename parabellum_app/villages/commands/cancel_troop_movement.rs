use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::army::Army;
use parabellum_types::errors::{AppError, GameError};
use uuid::Uuid;

use crate::villages::{
    VillageAggregate, VillageEvent, commands::as_domain_error, commands::as_invariant_error,
};

#[derive(Debug, Clone)]
pub struct CancelTroopMovement {
    pub movement_id: Uuid,
    pub arrival_action_id: Uuid,
    pub return_action_id: Uuid,
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub army: Army,
    pub returns_at: DateTime<Utc>,
}

impl Command for CancelTroopMovement {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.source_village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.source_village_id,
                actual: aggregate.aggregate_id(),
            }));
        }
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: self.source_village_id,
                player_id: self.player_id,
            }));
        }

        Ok(vec![VillageEvent::TroopMovementCanceled {
            movement_id: self.movement_id,
            arrival_action_id: self.arrival_action_id,
            return_action_id: self.return_action_id,
            army_id: self.army_id,
            player_id: self.player_id,
            source_village_id: self.source_village_id,
            target_village_id: self.target_village_id,
            army: self.army.clone(),
            returns_at: self.returns_at,
        }])
    }
}
