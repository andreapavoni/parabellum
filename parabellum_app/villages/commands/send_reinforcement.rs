use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::army::TroopSet;
use parabellum_types::buildings::BuildingName;
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
/// Schedules a reinforcement movement from source village to target village.
pub struct SendReinforcement {
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub target_village_id: u32,
    pub units: TroopSet,
    pub hero_id: Option<Uuid>,
    pub arrives_at: DateTime<Utc>,
}

impl Command for SendReinforcement {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        let source_village_id = aggregate.aggregate_id();
        let owner_id = aggregate.village().player_id();

        if owner_id != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: source_village_id,
                player_id: self.player_id,
            }));
        }

        if source_village_id == self.target_village_id {
            return Err(as_domain_error(GameError::VillageCannotTargetItself {
                village_id: source_village_id,
            }));
        }

        if aggregate.village().building_level(BuildingName::RallyPoint) == 0 {
            return Err(as_domain_error(GameError::BuildingRequirementsNotMet {
                building: BuildingName::RallyPoint,
                level: 1,
            }));
        }

        if self.units.immensity() == 0 && self.hero_id.is_none() {
            return Err(as_domain_error(GameError::NoUnitsSelected));
        }

        if !aggregate.village().has_units(&self.units) {
            return Err(as_domain_error(GameError::NotEnoughUnits));
        }

        Ok(vec![
            VillageEvent::VillageArmyDetached {
                army_id: self.army_id,
                units: self.units.clone(),
                hero_id: self.hero_id,
            },
            VillageEvent::ReinforcementSent {
                movement_id: self.movement_id,
                army_id: self.army_id,
                player_id: self.player_id,
                source_village_id,
                target_village_id: self.target_village_id,
                units: self.units.clone(),
                hero_id: self.hero_id,
                arrives_at: self.arrives_at,
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use mini_cqrs_es::{Aggregate, Command};
    use parabellum_game::models::{buildings::Building, village::VillageBuilding};
    use parabellum_types::{
        army::TroopSet,
        buildings::{BuildingGroup, BuildingName},
    };
    use uuid::Uuid;

    use crate::villages::{SendReinforcement, VillageAggregate, VillageEvent};

    fn rally_point(level: u8) -> VillageBuilding {
        VillageBuilding {
            slot_id: 39,
            building: Building {
                name: BuildingName::RallyPoint,
                group: BuildingGroup::Infrastructure,
                value: 0,
                population: 0,
                culture_points: 0,
                level,
            },
        }
    }

    #[tokio::test]
    async fn emits_reinforcement_events() {
        let player_id = Uuid::new_v4();
        let army_id = Uuid::new_v4();
        let movement_id = Uuid::new_v4();
        let mut aggregate = VillageAggregate::founded(10, player_id, vec![rally_point(1)]);
        aggregate
            .apply(&VillageEvent::UnitTrained {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 10,
                unit: parabellum_types::army::UnitName::Legionnaire,
                quantity_trained: 20,
            })
            .await;
        let units = TroopSet::new([12, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let arrives_at = Utc::now();

        let events = SendReinforcement {
            movement_id,
            army_id,
            player_id,
            target_village_id: 20,
            units: units.clone(),
            hero_id: None,
            arrives_at,
        }
        .handle(&aggregate)
        .await
        .unwrap();

        assert_eq!(
            events,
            vec![
                VillageEvent::VillageArmyDetached {
                    army_id,
                    units: units.clone(),
                    hero_id: None,
                },
                VillageEvent::ReinforcementSent {
                    movement_id,
                    army_id,
                    player_id,
                    source_village_id: 10,
                    target_village_id: 20,
                    units,
                    hero_id: None,
                    arrives_at,
                },
            ]
        );
    }

    #[tokio::test]
    async fn rejects_reinforcement_when_units_are_not_available() {
        let player_id = Uuid::new_v4();
        let aggregate = VillageAggregate::founded(10, player_id, vec![rally_point(1)]);

        let result = SendReinforcement {
            movement_id: Uuid::new_v4(),
            army_id: Uuid::new_v4(),
            player_id,
            target_village_id: 20,
            units: TroopSet::new([12, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            hero_id: None,
            arrives_at: Utc::now(),
        }
        .handle(&aggregate)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_reinforcement_without_rally_point() {
        let player_id = Uuid::new_v4();
        let aggregate = VillageAggregate::founded(10, player_id, vec![]);

        let result = SendReinforcement {
            movement_id: Uuid::new_v4(),
            army_id: Uuid::new_v4(),
            player_id,
            target_village_id: 20,
            units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            hero_id: None,
            arrives_at: Utc::now(),
        }
        .handle(&aggregate)
        .await;

        assert!(result.is_err());
    }
}
