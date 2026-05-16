use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::village::VillageBuilding;
use parabellum_types::errors::GameError;
use parabellum_types::map::Position;
use parabellum_types::tribe::Tribe;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
/// Initializes a village aggregate with immutable founding data and initial state.
pub struct FoundVillage {
    pub village_name: String,
    pub position: Position,
    pub tribe: Tribe,
    pub player_id: Uuid,
    pub parent_village_id: Option<u32>,
    pub buildings: Vec<VillageBuilding>,
}

impl Command for FoundVillage {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.version() != 0 {
            return Err(as_domain_error(GameError::VillageAlreadyFounded {
                village_id: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![VillageEvent::VillageFounded {
            village_id: aggregate.aggregate_id(),
            village_name: self.village_name.clone(),
            position: self.position.clone(),
            tribe: self.tribe.clone(),
            player_id: self.player_id,
            parent_village_id: self.parent_village_id,
            buildings: self.buildings.clone(),
        }])
    }
}

#[cfg(test)]
mod tests {
    use mini_cqrs_es::{Aggregate, Command};
    use parabellum_game::models::{buildings::Building, village::VillageBuilding};
    use parabellum_types::{
        buildings::{BuildingGroup, BuildingName},
        map::Position,
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::villages::{FoundVillage, VillageAggregate, VillageEvent};

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
    async fn emits_village_founded_event() {
        let player_id = Uuid::new_v4();
        let mut aggregate = VillageAggregate::default();
        aggregate.set_aggregate_id(99);

        let events = FoundVillage {
            village_name: "Village".to_string(),
            position: Position { x: 0, y: 0 },
            tribe: Tribe::Roman,
            player_id,
            parent_village_id: None,
            buildings: vec![rally_point(1)],
        }
        .handle(&aggregate)
        .await
        .unwrap();

        assert_eq!(
            events,
            vec![VillageEvent::VillageFounded {
                village_id: 99,
                village_name: "Village".to_string(),
                position: Position { x: 0, y: 0 },
                tribe: Tribe::Roman,
                player_id,
                parent_village_id: None,
                buildings: vec![rally_point(1)],
            }]
        );
    }
}
