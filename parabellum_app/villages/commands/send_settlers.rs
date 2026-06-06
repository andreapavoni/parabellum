use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::army::Army;
use parabellum_game::models::village::Village;
use parabellum_types::army::TroopSet;
use parabellum_types::buildings::BuildingName;
use parabellum_types::errors::GameError;
use parabellum_types::map::Position;
use parabellum_types::tribe::Tribe;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
pub struct SendSettlers {
    pub action_id: Uuid,
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub target_village_id: u32,
    pub target_position: Position,
    pub village_name: String,
    pub tribe: Tribe,
    pub arrives_at: DateTime<Utc>,
}

impl Command for SendSettlers {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        let source_village_id = aggregate.aggregate_id();
        if aggregate.village().player_id() != self.player_id {
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
        let settlers = aggregate.village().village.count_settlers_at_home();
        if settlers < 3 {
            return Err(as_domain_error(GameError::InsufficientSettlers));
        }
        let resources = Village::foundation_cost();
        if !aggregate.village().village.has_enough_resources(&resources) {
            return Err(as_domain_error(GameError::NotEnoughResources));
        }

        let units = TroopSet::new([0, 0, 0, 0, 0, 0, 0, 0, 0, 3]);
        let army = Army::new(
            Some(self.army_id),
            source_village_id,
            Some(self.target_village_id),
            self.player_id,
            aggregate.village().village.tribe.clone(),
            &units,
            aggregate.village().village.smithy(),
            None,
        );

        Ok(vec![
            VillageEvent::VillageArmyDetached { army: army.clone() },
            VillageEvent::SettlersSent {
                action_id: self.action_id,
                movement_id: self.movement_id,
                army_id: self.army_id,
                player_id: self.player_id,
                source_village_id,
                target_village_id: self.target_village_id,
                target_position: self.target_position.clone(),
                village_name: self.village_name.clone(),
                tribe: self.tribe.clone(),
                army,
                arrives_at: self.arrives_at,
            },
        ])
    }
}
