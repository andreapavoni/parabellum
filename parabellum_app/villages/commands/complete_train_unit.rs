use chrono::Duration;
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::{army::UnitName, errors::AppError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
/// Completes one scheduled training unit batch entry.
pub struct CompleteTrainUnit {
    pub action_id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub slot_id: u8,
    pub unit: UnitName,
    pub time_per_unit: i32,
    pub quantity_remaining: i32,
    pub execute_at: chrono::DateTime<chrono::Utc>,
}

impl Command for CompleteTrainUnit {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.village_id,
                actual: aggregate.aggregate_id(),
            }));
        }
        if self.quantity_remaining <= 0 {
            return Ok(vec![]);
        }

        let mut events = vec![VillageEvent::UnitTrained {
            action_id: self.action_id,
            player_id: self.player_id,
            village_id: self.village_id,
            unit: self.unit.clone(),
            quantity_trained: 1,
        }];

        let remaining_after = self.quantity_remaining - 1;
        if remaining_after > 0 {
            events.push(VillageEvent::UnitTrainingScheduled {
                action_id: Uuid::new_v4(),
                player_id: self.player_id,
                village_id: self.village_id,
                slot_id: self.slot_id,
                unit: self.unit.clone(),
                time_per_unit: self.time_per_unit,
                quantity_remaining: remaining_after,
                execute_at: self.execute_at + Duration::seconds(self.time_per_unit.max(1) as i64),
            });
        }

        Ok(events)
    }
}
