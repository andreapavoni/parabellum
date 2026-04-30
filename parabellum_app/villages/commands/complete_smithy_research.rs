use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::{army::UnitName, errors::AppError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
pub struct CompleteSmithyResearch {
    pub action_id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub unit: UnitName,
}

impl Command for CompleteSmithyResearch {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.village_id,
                actual: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![VillageEvent::SmithyResearchCompleted {
            action_id: self.action_id,
            player_id: self.player_id,
            village_id: self.village_id,
            unit: self.unit.clone(),
        }])
    }
}

#[cfg(test)]
mod tests {
    use mini_cqrs_es::{Aggregate, Command};
    use parabellum_types::{army::TroopSet, map::Position, tribe::Tribe};
    use uuid::Uuid;

    use crate::villages::{CompleteSmithyResearch, VillageAggregate, VillageEvent};

    #[tokio::test]
    async fn rejects_wrong_target() {
        let mut aggregate = VillageAggregate::default();
        let player_id = Uuid::new_v4();
        aggregate
            .apply(&VillageEvent::VillageFounded {
                village_id: 1,
                village_name: "v1".to_string(),
                position: Position { x: 0, y: 0 },
                tribe: Tribe::Roman,
                player_id,
                stationed_units: TroopSet::default(),
                buildings: vec![],
            })
            .await;

        let result = CompleteSmithyResearch {
            action_id: Uuid::new_v4(),
            player_id,
            village_id: 2,
            unit: parabellum_types::army::UnitName::Legionnaire,
        }
        .handle(&aggregate)
        .await;

        assert!(result.is_err());
    }
}
