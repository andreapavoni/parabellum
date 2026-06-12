use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::trapper::TrapperState;
use parabellum_types::{common::ResourceGroup, errors::AppError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
pub struct BuildTraps {
    pub action_id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub quantity_remaining: i32,
    pub time_per_trap: i32,
    pub cost: ResourceGroup,
    pub trapper: TrapperState,
    pub execute_at: DateTime<Utc>,
}

impl Command for BuildTraps {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.village_id,
                actual: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![VillageEvent::TrapBuildScheduled {
            action_id: self.action_id,
            player_id: self.player_id,
            village_id: self.village_id,
            quantity_remaining: self.quantity_remaining,
            time_per_trap: self.time_per_trap,
            cost: self.cost.clone(),
            trapper: self.trapper,
            execute_at: self.execute_at,
        }])
    }
}

#[derive(Debug, Clone)]
pub struct CompleteTrapBuild {
    pub action_id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub quantity_built: u32,
    pub trapper: TrapperState,
}

impl Command for CompleteTrapBuild {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.village_id,
                actual: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![VillageEvent::TrapBuilt {
            action_id: self.action_id,
            player_id: self.player_id,
            village_id: self.village_id,
            quantity_built: self.quantity_built,
            trapper: self.trapper,
        }])
    }
}
