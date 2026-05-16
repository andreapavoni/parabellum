use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::army::Army;
use parabellum_types::army::TroopSet;
use parabellum_types::errors::{AppError, GameError};
use uuid::Uuid;

use crate::villages::{
    VillageAggregate, VillageEvent, commands::as_domain_error, commands::as_invariant_error,
};

#[derive(Debug, Clone)]
pub struct ReleaseReinforcements {
    pub action_id: Uuid,
    pub movement_id: Uuid,
    pub player_id: Uuid,
    pub stationed_village_id: u32,
    pub home_village_id: u32,
    pub reinforcement_army: Army,
    pub units: TroopSet,
    pub hero_id: Option<Uuid>,
    pub returns_at: DateTime<Utc>,
}

impl Command for ReleaseReinforcements {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.stationed_village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.stationed_village_id,
                actual: aggregate.aggregate_id(),
            }));
        }
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: self.stationed_village_id,
                player_id: self.player_id,
            }));
        }
        if self.units.immensity() == 0 {
            return Err(as_domain_error(GameError::NoUnitsSelected));
        }

        if !has_units(&self.reinforcement_army, &self.units) {
            return Err(as_domain_error(GameError::NotEnoughUnits));
        }

        let mut return_army = self.reinforcement_army.clone();
        return_army.update_units(&self.units);
        match (self.hero_id, self.reinforcement_army.hero()) {
            (Some(hero_id), Some(hero)) if hero.id == hero_id => {
                return_army.set_hero(Some(hero));
            }
            (Some(hero_id), _) => {
                return Err(as_domain_error(GameError::HeroNotAtHome {
                    hero_id,
                    village_id: self.stationed_village_id,
                }));
            }
            (None, _) => {
                return_army.set_hero(None);
            }
        }

        Ok(vec![VillageEvent::ReinforcementsReleased {
            action_id: self.action_id,
            movement_id: self.movement_id,
            army_id: self.reinforcement_army.id,
            player_id: self.player_id,
            home_village_id: self.home_village_id,
            stationed_village_id: self.stationed_village_id,
            army: return_army,
            returns_at: self.returns_at,
        }])
    }
}

fn has_units(army: &Army, units: &TroopSet) -> bool {
    army.units()
        .units()
        .iter()
        .zip(units.units().iter())
        .all(|(available, requested)| available >= requested)
}
