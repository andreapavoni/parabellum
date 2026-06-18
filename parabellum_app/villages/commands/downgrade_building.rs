use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
/// Schedules a building level downgrade for an existing slot.
pub struct DowngradeBuilding {
    pub player_id: Uuid,
    pub slot_id: u8,
    pub speed: i8,
}

impl Command for DowngradeBuilding {
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
            .schedule_downgrade_building(self.slot_id, self.speed)
            .map_err(as_domain_error)?;
        let execute_at = aggregate
            .village()
            .next_execution_time_for_slot(self.slot_id, duration_secs);

        Ok(vec![VillageEvent::BuildingDowngradeScheduled {
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

    use crate::villages::{DowngradeBuilding, VillageAggregate, VillageEvent};

    fn aggregate_with_main_and_target(main_level: u8, target_level: u8) -> VillageAggregate {
        VillageAggregate::founded(
            1,
            Uuid::new_v4(),
            vec![
                VillageBuilding {
                    slot_id: 19,
                    building: Building {
                        name: BuildingName::MainBuilding,
                        group: BuildingGroup::Infrastructure,
                        value: 0,
                        population: 0,
                        culture_points: 0,
                        level: main_level,
                    },
                },
                VillageBuilding {
                    slot_id: 22,
                    building: Building {
                        name: BuildingName::Cranny,
                        group: BuildingGroup::Infrastructure,
                        value: 0,
                        population: 0,
                        culture_points: 0,
                        level: target_level,
                    },
                },
            ],
        )
    }

    #[tokio::test]
    async fn rejects_downgrade_when_main_building_below_10() {
        let aggregate = aggregate_with_main_and_target(3, 2);
        let result = DowngradeBuilding {
            player_id: aggregate.player_id(),
            slot_id: 22,
            speed: 1,
        }
        .handle(&aggregate)
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn schedules_downgrade_when_main_building_is_10() {
        let aggregate = aggregate_with_main_and_target(10, 2);
        let result = DowngradeBuilding {
            player_id: aggregate.player_id(),
            slot_id: 22,
            speed: 1,
        }
        .handle(&aggregate)
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn rejects_downgrade_for_resource_fields() {
        let player_id = Uuid::new_v4();
        let aggregate = VillageAggregate::founded(
            1,
            player_id,
            vec![
                VillageBuilding {
                    slot_id: 1,
                    building: Building {
                        name: BuildingName::Woodcutter,
                        group: BuildingGroup::Resources,
                        value: 0,
                        population: 0,
                        culture_points: 0,
                        level: 1,
                    },
                },
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
            ],
        );

        let result = DowngradeBuilding {
            player_id,
            slot_id: 1,
            speed: 1,
        }
        .handle(&aggregate)
        .await;

        assert!(result.is_err());
    }

    async fn aggregate_with_tribe(tribe: Tribe) -> VillageAggregate {
        let mut aggregate = VillageAggregate::default();
        let player_id = Uuid::new_v4();
        aggregate
            .apply(&VillageEvent::VillageFounded {
                village_id: 1,
                village_name: "village-1".to_string(),
                position: Position { x: 0, y: 0 },
                tribe,
                player_id,
                parent_village_id: None,
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
                        slot_id: 22,
                        building: Building {
                            name: BuildingName::Cranny,
                            group: BuildingGroup::Infrastructure,
                            value: 0,
                            population: 0,
                            culture_points: 0,
                            level: 3,
                        },
                    },
                ],
            })
            .await;
        aggregate
    }

    #[tokio::test]
    async fn rejects_downgrade_when_non_roman_queue_is_full() {
        let mut aggregate = aggregate_with_tribe(Tribe::Gaul).await;
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
                cost: parabellum_types::common::ResourceGroup::new(0, 0, 0, 0),
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
                cost: parabellum_types::common::ResourceGroup::new(0, 0, 0, 0),
                execute_at: now + Duration::minutes(2),
            })
            .await;

        let result = DowngradeBuilding {
            player_id,
            slot_id: 22,
            speed: 1,
        }
        .handle(&aggregate)
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn schedules_downgrade_after_existing_slot_queue() {
        let mut aggregate = aggregate_with_tribe(Tribe::Roman).await;
        let player_id = aggregate.player_id();
        let first_eta = Utc::now() + Duration::minutes(4);

        aggregate
            .apply(&VillageEvent::BuildingDowngradeScheduled {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 1,
                slot_id: 22,
                building_name: BuildingName::Cranny,
                level: 2,
                speed: 1,
                execute_at: first_eta,
            })
            .await;

        let events = DowngradeBuilding {
            player_id,
            slot_id: 22,
            speed: 1,
        }
        .handle(&aggregate)
        .await
        .unwrap();

        let VillageEvent::BuildingDowngradeScheduled { execute_at, .. } = &events[0] else {
            panic!("expected BuildingDowngradeScheduled");
        };
        assert!(*execute_at > first_eta);
    }
}
