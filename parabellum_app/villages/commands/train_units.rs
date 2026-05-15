use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::{buildings::BuildingName, errors::GameError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
/// Schedules unit training according to village queue rules.
pub struct TrainUnits {
    pub player_id: Uuid,
    pub unit_idx: u8,
    pub building_name: BuildingName,
    pub quantity: i32,
    pub speed: i8,
}

impl Command for TrainUnits {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: aggregate.aggregate_id(),
                player_id: self.player_id,
            }));
        }
        if self.quantity <= 0 {
            return Err(as_domain_error(GameError::InvalidUnitQuantity(
                self.quantity,
            )));
        }

        let (slot_id, unit, time_per_unit, cost) = aggregate
            .village()
            .schedule_train_units(
                self.unit_idx,
                self.building_name.clone(),
                self.quantity,
                self.speed,
            )
            .map_err(as_domain_error)?;

        let execute_at = aggregate
            .village()
            .next_execution_time_for_training_slot(slot_id, time_per_unit as i64);

        Ok(vec![VillageEvent::UnitTrainingScheduled {
            action_id: Uuid::new_v4(),
            player_id: self.player_id,
            village_id: aggregate.aggregate_id(),
            slot_id,
            unit,
            time_per_unit,
            quantity_remaining: self.quantity,
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
    use parabellum_types::{
        army::UnitName,
        buildings::{BuildingGroup, BuildingName},
        map::Position,
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::villages::{TrainUnits, VillageAggregate, VillageEvent};

    fn building(slot_id: u8, name: BuildingName, level: u8, speed: i8) -> VillageBuilding {
        VillageBuilding {
            slot_id,
            building: Building::new(name, speed)
                .at_level(level, speed)
                .expect("building data should be available for train unit tests"),
        }
    }

    async fn training_ready_aggregate() -> VillageAggregate {
        let mut aggregate = VillageAggregate::default();
        let player_id = Uuid::new_v4();
        aggregate
            .apply(&VillageEvent::VillageFounded {
                village_id: 1,
                village_name: "v1".to_string(),
                position: Position { x: 0, y: 0 },
                tribe: Tribe::Roman,
                player_id,
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
                            value: 1000,
                            population: 0,
                            culture_points: 0,
                            level: 1,
                        },
                    },
                ],
            })
            .await;
        aggregate.set_resources_for_test(parabellum_types::common::ResourceGroup(
            20_000, 20_000, 20_000, 20_000,
        ));
        aggregate
    }

    async fn expansion_ready_aggregate(expansion_building: VillageBuilding) -> VillageAggregate {
        let mut aggregate = VillageAggregate::default();
        let player_id = Uuid::new_v4();
        aggregate
            .apply(&VillageEvent::VillageFounded {
                village_id: 1,
                village_name: "v1".to_string(),
                position: Position { x: 0, y: 0 },
                tribe: Tribe::Roman,
                player_id,
                buildings: vec![
                    building(19, BuildingName::MainBuilding, 1, 1),
                    expansion_building,
                    building(26, BuildingName::Warehouse, 20, 1),
                    building(27, BuildingName::Granary, 20, 1),
                ],
            })
            .await;
        aggregate.set_resources_for_test(parabellum_types::common::ResourceGroup(
            80_000, 80_000, 80_000, 80_000,
        ));
        aggregate.set_academy_research_for_test(&UnitName::Senator, true);
        aggregate
    }

    #[tokio::test]
    async fn rejects_invalid_quantity() {
        let aggregate = training_ready_aggregate().await;
        let result = TrainUnits {
            player_id: aggregate.player_id(),
            unit_idx: 0,
            building_name: BuildingName::Barracks,
            quantity: 0,
            speed: 1,
        }
        .handle(&aggregate)
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_wrong_owner() {
        let aggregate = training_ready_aggregate().await;
        let result = TrainUnits {
            player_id: Uuid::new_v4(),
            unit_idx: 0,
            building_name: BuildingName::Barracks,
            quantity: 1,
            speed: 1,
        }
        .handle(&aggregate)
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_invalid_building_for_unit() {
        let aggregate = training_ready_aggregate().await;
        let result = TrainUnits {
            player_id: aggregate.player_id(),
            unit_idx: 0,
            building_name: BuildingName::Stable,
            quantity: 1,
            speed: 1,
        }
        .handle(&aggregate)
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_settlers_without_foundation_slots() {
        let aggregate = training_ready_aggregate().await;
        let result = TrainUnits {
            player_id: aggregate.player_id(),
            unit_idx: 9,
            building_name: BuildingName::Residence,
            quantity: 3,
            speed: 1,
        }
        .handle(&aggregate)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn allows_three_settlers_with_residence_level_10() {
        let aggregate =
            expansion_ready_aggregate(building(25, BuildingName::Residence, 10, 1)).await;

        let events = TrainUnits {
            player_id: aggregate.player_id(),
            unit_idx: 9,
            building_name: BuildingName::Residence,
            quantity: 3,
            speed: 1,
        }
        .handle(&aggregate)
        .await
        .unwrap();

        assert!(matches!(
            events.first(),
            Some(VillageEvent::UnitTrainingScheduled {
                unit: UnitName::Settler,
                quantity_remaining: 3,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn rejects_four_settlers_with_one_residence_slot() {
        let aggregate =
            expansion_ready_aggregate(building(25, BuildingName::Residence, 10, 1)).await;

        let result = TrainUnits {
            player_id: aggregate.player_id(),
            unit_idx: 9,
            building_name: BuildingName::Residence,
            quantity: 4,
            speed: 1,
        }
        .handle(&aggregate)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_chief_when_settlers_use_only_residence_slot() {
        let mut aggregate =
            expansion_ready_aggregate(building(25, BuildingName::Residence, 10, 1)).await;
        let player_id = aggregate.player_id();
        aggregate
            .apply(&VillageEvent::UnitTrained {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 1,
                unit: UnitName::Settler,
                quantity_trained: 3,
            })
            .await;

        let result = TrainUnits {
            player_id,
            unit_idx: 8,
            building_name: BuildingName::Residence,
            quantity: 1,
            speed: 1,
        }
        .handle(&aggregate)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_settlers_when_queued_settlers_fill_residence_slot() {
        let mut aggregate =
            expansion_ready_aggregate(building(25, BuildingName::Residence, 10, 1)).await;
        let player_id = aggregate.player_id();
        aggregate
            .apply(&VillageEvent::UnitTrainingScheduled {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 1,
                slot_id: 25,
                unit: UnitName::Settler,
                time_per_unit: 60,
                quantity_remaining: 3,
                cost: parabellum_types::common::ResourceGroup::new(0, 0, 0, 0),
                execute_at: Utc::now() + Duration::minutes(2),
            })
            .await;

        let result = TrainUnits {
            player_id,
            unit_idx: 9,
            building_name: BuildingName::Residence,
            quantity: 1,
            speed: 1,
        }
        .handle(&aggregate)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn sequences_same_slot_after_queued_training() {
        let mut aggregate = training_ready_aggregate().await;
        let player_id = aggregate.player_id();
        let first_eta = Utc::now() + Duration::minutes(2);
        aggregate
            .apply(&VillageEvent::UnitTrainingScheduled {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 1,
                slot_id: 20,
                unit: parabellum_types::army::UnitName::Legionnaire,
                time_per_unit: 60,
                quantity_remaining: 2,
                cost: parabellum_types::common::ResourceGroup::new(0, 0, 0, 0),
                execute_at: first_eta,
            })
            .await;

        let events = TrainUnits {
            player_id,
            unit_idx: 0,
            building_name: BuildingName::Barracks,
            quantity: 1,
            speed: 1,
        }
        .handle(&aggregate)
        .await
        .unwrap();

        let VillageEvent::UnitTrainingScheduled { execute_at, .. } = &events[0] else {
            panic!("expected UnitTrainingScheduled");
        };
        assert!(*execute_at > first_eta);
    }
}
