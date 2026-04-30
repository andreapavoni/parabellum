use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::{army::UnitName, errors::GameError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
/// Schedules a smithy research action for a unit.
pub struct ResearchSmithy {
    pub player_id: Uuid,
    pub unit: UnitName,
    pub speed: i8,
}

impl Command for ResearchSmithy {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: aggregate.aggregate_id(),
                player_id: self.player_id,
            }));
        }

        let duration_secs = aggregate
            .village()
            .schedule_smithy_research(self.unit.clone(), self.speed)
            .map_err(as_domain_error)?;
        let execute_at = aggregate
            .village()
            .next_execution_time_for_smithy(duration_secs);

        Ok(vec![VillageEvent::SmithyResearchScheduled {
            action_id: Uuid::new_v4(),
            player_id: self.player_id,
            village_id: aggregate.aggregate_id(),
            unit: self.unit.clone(),
            execute_at,
        }])
    }
}

#[cfg(test)]
mod tests {
    use mini_cqrs_es::{Aggregate, Command};
    use parabellum_game::models::{buildings::Building, village::VillageBuilding};
    use parabellum_types::{
        army::TroopSet,
        buildings::{BuildingGroup, BuildingName},
        map::Position,
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::villages::{ResearchSmithy, VillageAggregate, VillageEvent};

    async fn smithy_ready_aggregate() -> VillageAggregate {
        let mut aggregate = VillageAggregate::default();
        let player_id = Uuid::new_v4();
        aggregate
            .apply(&VillageEvent::VillageFounded {
                village_id: 1,
                village_name: "v1".to_string(),
                position: Position { x: 0, y: 0 },
                tribe: Tribe::Teuton,
                player_id,
                stationed_units: TroopSet::default(),
                buildings: vec![
                    VillageBuilding {
                        slot_id: 19,
                        building: Building {
                            name: BuildingName::MainBuilding,
                            group: BuildingGroup::Infrastructure,
                            value: 0,
                            population: 0,
                            culture_points: 0,
                            level: 1,
                        },
                    },
                    VillageBuilding {
                        slot_id: 20,
                        building: Building {
                            name: BuildingName::Barracks,
                            group: BuildingGroup::Military,
                            value: 0,
                            population: 0,
                            culture_points: 0,
                            level: 1,
                        },
                    },
                    VillageBuilding {
                        slot_id: 23,
                        building: Building {
                            name: BuildingName::Smithy,
                            group: BuildingGroup::Infrastructure,
                            value: 0,
                            population: 0,
                            culture_points: 0,
                            level: 1,
                        },
                    },
                ],
            })
            .await;
        aggregate.set_resources_for_test(parabellum_types::common::ResourceGroup(
            500_000, 500_000, 500_000, 500_000,
        ));
        aggregate
    }

    #[tokio::test]
    async fn rejects_wrong_owner() {
        let aggregate = smithy_ready_aggregate().await;
        let result = ResearchSmithy {
            player_id: Uuid::new_v4(),
            unit: parabellum_types::army::UnitName::Maceman,
            speed: 1,
        }
        .handle(&aggregate)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn schedules_smithy_research() {
        let aggregate = smithy_ready_aggregate().await;
        let result = ResearchSmithy {
            player_id: aggregate.player_id(),
            unit: parabellum_types::army::UnitName::Maceman,
            speed: 1,
        }
        .handle(&aggregate)
        .await
        .unwrap();

        assert!(matches!(
            result.first(),
            Some(VillageEvent::SmithyResearchScheduled { .. })
        ));
    }
}
