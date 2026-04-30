use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
pub struct UpgradeBuilding {
    pub player_id: Uuid,
    pub slot_id: u8,
    pub speed: i8,
}

impl Command for UpgradeBuilding {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: aggregate.aggregate_id(),
                player_id: self.player_id,
            }));
        }
        let (building_name, next_level, duration_secs) = aggregate
            .village()
            .schedule_upgrade_building(self.slot_id, self.speed)
            .map_err(as_domain_error)?;
        let execute_at = aggregate
            .village()
            .next_execution_time_for_slot(self.slot_id, duration_secs);

        Ok(vec![VillageEvent::BuildingUpgradeScheduled {
            action_id: Uuid::new_v4(),
            player_id: self.player_id,
            village_id: aggregate.aggregate_id(),
            slot_id: self.slot_id,
            building_name,
            level: next_level,
            speed: self.speed,
            execute_at,
        }])
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use mini_cqrs_es::Aggregate;
    use mini_cqrs_es::Command;
    use parabellum_game::models::{buildings::Building, village::VillageBuilding};
    use parabellum_types::{
        buildings::{BuildingGroup, BuildingName},
        map::Position,
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::villages::{UpgradeBuilding, VillageAggregate, VillageEvent};

    #[tokio::test]
    async fn rejects_upgrade_on_empty_slot() {
        let aggregate = VillageAggregate::founded(1, Uuid::new_v4(), Default::default(), vec![]);
        let result = UpgradeBuilding {
            player_id: aggregate.player_id(),
            slot_id: 22,
            speed: 1,
        }
        .handle(&aggregate)
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn schedules_upgrade_on_existing_building() {
        let player_id = Uuid::new_v4();
        let aggregate = VillageAggregate::founded(
            1,
            player_id,
            Default::default(),
            vec![VillageBuilding {
                slot_id: 19,
                building: Building {
                    name: BuildingName::MainBuilding,
                    group: BuildingGroup::Infrastructure,
                    value: 0,
                    population: 0,
                    culture_points: 0,
                    level: 3,
                },
            }],
        );
        let result = UpgradeBuilding {
            player_id,
            slot_id: 19,
            speed: 1,
        }
        .handle(&aggregate)
        .await;
        assert!(result.is_ok());
    }

    async fn aggregate_with_tribe_and_building(
        tribe: Tribe,
        building_slot: u8,
        building_name: BuildingName,
        building_level: u8,
    ) -> VillageAggregate {
        let mut aggregate = VillageAggregate::default();
        let player_id = Uuid::new_v4();
        aggregate
            .apply(&VillageEvent::VillageFounded {
                village_id: 1,
                village_name: "village-1".to_string(),
                position: Position { x: 0, y: 0 },
                tribe,
                player_id,
                stationed_units: Default::default(),
                buildings: vec![
                    VillageBuilding {
                        slot_id: 19,
                        building: Building {
                            name: BuildingName::MainBuilding,
                            group: BuildingGroup::Infrastructure,
                            value: 0,
                            population: 0,
                            culture_points: 0,
                            level: 10,
                        },
                    },
                    VillageBuilding {
                        slot_id: building_slot,
                        building: Building {
                            name: building_name,
                            group: BuildingGroup::Infrastructure,
                            value: 0,
                            population: 0,
                            culture_points: 0,
                            level: building_level,
                        },
                    },
                ],
            })
            .await;
        aggregate
    }

    #[tokio::test]
    async fn rejects_upgrade_when_non_roman_queue_is_full() {
        let mut aggregate =
            aggregate_with_tribe_and_building(Tribe::Gaul, 22, BuildingName::Cranny, 1).await;
        let player_id = aggregate.player_id();
        let now = Utc::now();

        aggregate
            .apply(&VillageEvent::BuildingConstructionScheduled {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 1,
                slot_id: 21,
                building_name: BuildingName::Granary,
                level: 1,
                speed: 1,
                execute_at: now + Duration::minutes(1),
            })
            .await;
        aggregate
            .apply(&VillageEvent::BuildingConstructionScheduled {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 1,
                slot_id: 23,
                building_name: BuildingName::Warehouse,
                level: 1,
                speed: 1,
                execute_at: now + Duration::minutes(2),
            })
            .await;

        let result = UpgradeBuilding {
            player_id,
            slot_id: 22,
            speed: 1,
        }
        .handle(&aggregate)
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn schedules_upgrade_after_existing_slot_queue() {
        let mut aggregate =
            aggregate_with_tribe_and_building(Tribe::Roman, 22, BuildingName::Cranny, 2).await;
        let player_id = aggregate.player_id();
        let first_eta = Utc::now() + Duration::minutes(5);

        aggregate
            .apply(&VillageEvent::BuildingUpgradeScheduled {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 1,
                slot_id: 22,
                building_name: BuildingName::Cranny,
                level: 3,
                speed: 1,
                execute_at: first_eta,
            })
            .await;

        let events = UpgradeBuilding {
            player_id,
            slot_id: 22,
            speed: 1,
        }
        .handle(&aggregate)
        .await
        .unwrap();

        let VillageEvent::BuildingUpgradeScheduled { execute_at, .. } = &events[0] else {
            panic!("expected BuildingUpgradeScheduled");
        };
        assert!(*execute_at > first_eta);
    }
}
