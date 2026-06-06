use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::army::TroopSet;
use uuid::Uuid;

use crate::villages::{
    ArmyDispatch, ArmyDispatchRequest, VillageAggregate, VillageEvent, commands::as_domain_error,
};

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
        let detached_army = ArmyDispatch::detach_from_home(
            aggregate.village(),
            ArmyDispatchRequest {
                army_id: self.army_id,
                source_village_id,
                target_village_id: self.target_village_id,
                player_id: self.player_id,
                units: self.units.clone(),
                hero_id: self.hero_id,
                allow_hero: true,
                scouts_only: false,
            },
        )
        .map_err(as_domain_error)?;
        Ok(vec![
            VillageEvent::VillageArmyDetached {
                army: detached_army.clone(),
            },
            VillageEvent::ReinforcementSent {
                movement_id: self.movement_id,
                army_id: self.army_id,
                player_id: self.player_id,
                source_village_id,
                target_village_id: self.target_village_id,
                army: detached_army,
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

        assert!(matches!(
            events.first(),
            Some(VillageEvent::VillageArmyDetached { army }) if army.id == army_id
        ));
        assert!(matches!(
            events.get(1),
            Some(VillageEvent::ReinforcementSent {
                movement_id: m,
                army_id: a,
                player_id: p,
                source_village_id: 10,
                target_village_id: 20,
                arrives_at: at,
                ..
            }) if *m == movement_id && *a == army_id && *p == player_id && *at == arrives_at
        ));
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
