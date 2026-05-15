use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::{buildings::BuildingName, errors::GameError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
/// Schedules construction of a new building at a given slot.
pub struct AddBuilding {
    pub player_id: Uuid,
    pub slot_id: u8,
    pub building_name: BuildingName,
    pub speed: i8,
}

impl Command for AddBuilding {
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
            .schedule_add_building(self.slot_id, self.building_name.clone(), self.speed)
            .map_err(as_domain_error)?;
        let execute_at = aggregate
            .village()
            .next_execution_time_for_slot(self.slot_id, duration_secs);

        Ok(vec![VillageEvent::BuildingConstructionScheduled {
            action_id: Uuid::new_v4(),
            player_id: self.player_id,
            village_id: aggregate.aggregate_id(),
            slot_id: self.slot_id,
            building_name: self.building_name.clone(),
            level: 1,
            speed: self.speed,
            cost,
            execute_at,
        }])
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use mini_cqrs_es::{Aggregate, Command};
    use parabellum_game::models::{buildings::Building, village::VillageBuilding};
    use parabellum_types::{buildings::BuildingName, map::Position, tribe::Tribe};
    use uuid::Uuid;

    use crate::villages::{AddBuilding, VillageAggregate, VillageEvent};

    async fn aggregate_with_tribe(tribe: Tribe) -> VillageAggregate {
        let mut aggregate = VillageAggregate::default();
        aggregate
            .apply(&VillageEvent::VillageFounded {
                village_id: 1,
                village_name: "village-1".to_string(),
                position: Position { x: 0, y: 0 },
                tribe,
                player_id: Uuid::new_v4(),
                buildings: vec![VillageBuilding {
                    slot_id: 19,
                    building: Building {
                        name: BuildingName::MainBuilding,
                        group: parabellum_types::buildings::BuildingGroup::Infrastructure,
                        value: 0,
                        population: 0,
                        culture_points: 0,
                        level: 1,
                    },
                }],
            })
            .await;
        aggregate
    }

    #[tokio::test]
    async fn rejects_add_building_when_non_roman_queue_is_full() {
        let mut aggregate = aggregate_with_tribe(Tribe::Gaul).await;
        let player_id = aggregate.player_id();
        let now = Utc::now();

        aggregate
            .apply(&VillageEvent::BuildingConstructionScheduled {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 1,
                slot_id: 21,
                building_name: BuildingName::Cranny,
                level: 1,
                speed: 1,
                cost: parabellum_types::common::ResourceGroup::new(0, 0, 0, 0),
                execute_at: now + Duration::minutes(1),
            })
            .await;
        aggregate
            .apply(&VillageEvent::BuildingConstructionScheduled {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 1,
                slot_id: 22,
                building_name: BuildingName::Granary,
                level: 1,
                speed: 1,
                cost: parabellum_types::common::ResourceGroup::new(0, 0, 0, 0),
                execute_at: now + Duration::minutes(2),
            })
            .await;

        let result = AddBuilding {
            player_id,
            slot_id: 23,
            building_name: BuildingName::Warehouse,
            speed: 1,
        }
        .handle(&aggregate)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_add_building_with_queued_conflict() {
        let mut aggregate = aggregate_with_tribe(Tribe::Roman).await;
        let player_id = aggregate.player_id();

        aggregate
            .apply(&VillageEvent::BuildingConstructionScheduled {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 1,
                slot_id: 22,
                building_name: BuildingName::Palace,
                level: 1,
                speed: 1,
                cost: parabellum_types::common::ResourceGroup::new(0, 0, 0, 0),
                execute_at: Utc::now() + Duration::minutes(1),
            })
            .await;

        let result = AddBuilding {
            player_id,
            slot_id: 23,
            building_name: BuildingName::Residence,
            speed: 1,
        }
        .handle(&aggregate)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn schedules_same_slot_after_existing_slot_queue() {
        let mut aggregate = aggregate_with_tribe(Tribe::Roman).await;
        let player_id = aggregate.player_id();
        let first_eta = Utc::now() + Duration::minutes(5);

        aggregate
            .apply(&VillageEvent::BuildingConstructionScheduled {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 1,
                slot_id: 22,
                building_name: BuildingName::Cranny,
                level: 1,
                speed: 1,
                cost: parabellum_types::common::ResourceGroup::new(0, 0, 0, 0),
                execute_at: first_eta,
            })
            .await;

        let events = AddBuilding {
            player_id,
            slot_id: 22,
            building_name: BuildingName::Granary,
            speed: 1,
        }
        .handle(&aggregate)
        .await
        .unwrap();

        let VillageEvent::BuildingConstructionScheduled { execute_at, .. } = &events[0] else {
            panic!("expected BuildingConstructionScheduled");
        };
        assert!(*execute_at > first_eta);
    }
}
