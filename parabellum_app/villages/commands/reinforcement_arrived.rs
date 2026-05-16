use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::army::Army;
use parabellum_types::errors::AppError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
/// Completes a scheduled reinforcement arrival.
pub struct ReinforcementArrived {
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub army: Army,
    pub arrives_at: DateTime<Utc>,
}

impl Command for ReinforcementArrived {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.source_village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.source_village_id,
                actual: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![VillageEvent::ReinforcementArrived {
            movement_id: self.movement_id,
            army_id: self.army_id,
            player_id: self.player_id,
            source_village_id: self.source_village_id,
            target_village_id: self.target_village_id,
            army: self.army.clone(),
            arrives_at: self.arrives_at,
        }])
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use mini_cqrs_es::Command;
    use uuid::Uuid;

    use crate::villages::{ReinforcementArrived, VillageAggregate, VillageEvent};

    #[tokio::test]
    async fn emits_reinforcement_arrived_event() {
        let aggregate = VillageAggregate::default();
        let command = ReinforcementArrived {
            movement_id: Uuid::new_v4(),
            army_id: Uuid::new_v4(),
            player_id: Uuid::new_v4(),
            source_village_id: 0,
            target_village_id: 2,
            army: parabellum_game::models::army::Army::new(
                Some(Uuid::new_v4()),
                0,
                Some(2),
                Uuid::new_v4(),
                parabellum_types::tribe::Tribe::Roman,
                &parabellum_types::army::TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                &[0, 0, 0, 0, 0, 0, 0, 0],
                None,
            ),
            arrives_at: Utc::now(),
        };

        let events = command.handle(&aggregate).await.unwrap();
        assert!(matches!(
            events.first(),
            Some(VillageEvent::ReinforcementArrived { .. })
        ));
    }
}
