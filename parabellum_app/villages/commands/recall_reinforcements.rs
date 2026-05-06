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
pub struct RecallReinforcements {
    pub action_id: Uuid,
    pub movement_id: Uuid,
    pub player_id: Uuid,
    pub home_village_id: u32,
    pub stationed_village_id: u32,
    pub reinforcement_army: Army,
    pub units: TroopSet,
    pub returns_at: DateTime<Utc>,
}

impl Command for RecallReinforcements {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.home_village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.home_village_id,
                actual: aggregate.aggregate_id(),
            }));
        }
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: self.home_village_id,
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

        Ok(vec![VillageEvent::ReinforcementsRecalled {
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
