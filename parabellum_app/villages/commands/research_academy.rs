use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::{army::UnitName, errors::GameError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
/// Schedules an academy research action for a unit.
pub struct ResearchAcademy {
    pub player_id: Uuid,
    pub unit: UnitName,
    pub speed: i8,
}

impl Command for ResearchAcademy {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: aggregate.aggregate_id(),
                player_id: self.player_id,
            }));
        }

        let (duration_secs, cost) = aggregate
            .village()
            .schedule_academy_research(self.unit.clone(), self.speed)
            .map_err(as_domain_error)?;
        let execute_at = aggregate
            .village()
            .next_execution_time_for_academy(duration_secs);

        Ok(vec![VillageEvent::AcademyResearchScheduled {
            action_id: Uuid::new_v4(),
            player_id: self.player_id,
            village_id: aggregate.aggregate_id(),
            unit: self.unit.clone(),
            cost,
            execute_at,
        }])
    }
}

#[cfg(test)]
mod tests {
    use mini_cqrs_es::{Aggregate, Command};
    use parabellum_types::{map::Position, tribe::Tribe};
    use uuid::Uuid;

    use crate::villages::{ResearchAcademy, VillageAggregate, VillageEvent};

    #[tokio::test]
    async fn rejects_wrong_owner() {
        let mut aggregate = VillageAggregate::default();
        let owner_id = Uuid::new_v4();
        aggregate
            .apply(&VillageEvent::VillageFounded {
                village_id: 1,
                village_name: "v1".to_string(),
                position: Position { x: 0, y: 0 },
                tribe: Tribe::Roman,
                player_id: owner_id,
                parent_village_id: None,
                buildings: vec![],
            })
            .await;

        let result = ResearchAcademy {
            player_id: Uuid::new_v4(),
            unit: parabellum_types::army::UnitName::Legionnaire,
            speed: 1,
        }
        .handle(&aggregate)
        .await;

        assert!(result.is_err());
    }
}
