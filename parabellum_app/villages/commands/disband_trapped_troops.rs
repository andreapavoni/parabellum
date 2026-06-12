use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::trapper::TrapperState;
use parabellum_types::errors::AppError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
pub struct DisbandTrappedTroops {
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub home_village_id: u32,
    pub trapped_village_id: u32,
    pub trapper: TrapperState,
}

impl Command for DisbandTrappedTroops {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.trapped_village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.trapped_village_id,
                actual: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![VillageEvent::TrappedTroopsDisbanded {
            army_id: self.army_id,
            player_id: self.player_id,
            home_village_id: self.home_village_id,
            trapped_village_id: self.trapped_village_id,
            trapper: self.trapper,
        }])
    }
}
