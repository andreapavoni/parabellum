use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::army::Army;
use parabellum_types::battle::AttackType;
use parabellum_types::buildings::BuildingName;
use parabellum_types::errors::AppError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
/// Completes attack arrival when scheduled attack reaches target village.
pub struct CompleteAttackArrival {
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub action_id: Uuid,
    pub return_action_id: Uuid,
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub army: Army,
    pub attack_type: AttackType,
    pub catapult_targets: [BuildingName; 2],
    pub arrives_at: DateTime<Utc>,
    pub returns_at: DateTime<Utc>,
}

impl Command for CompleteAttackArrival {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.source_village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.source_village_id,
                actual: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![VillageEvent::AttackArrived {
            movement_id: self.movement_id,
            army_id: self.army_id,
            action_id: self.action_id,
            return_action_id: self.return_action_id,
            player_id: self.player_id,
            source_village_id: self.source_village_id,
            target_village_id: self.target_village_id,
            army: self.army.clone(),
            attack_type: self.attack_type.clone(),
            catapult_targets: self.catapult_targets.clone(),
            arrives_at: self.arrives_at,
            returns_at: self.returns_at,
        }])
    }
}
