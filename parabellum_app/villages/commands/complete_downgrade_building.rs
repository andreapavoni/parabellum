use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::{buildings::BuildingName, errors::AppError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
/// Completes a previously scheduled building downgrade action.
pub struct CompleteDowngradeBuilding {
    pub action_id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub slot_id: u8,
    pub building_name: BuildingName,
    pub level: u8,
    pub speed: i8,
}

impl Command for CompleteDowngradeBuilding {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.village_id,
                actual: aggregate.aggregate_id(),
            }));
        }
        Ok(vec![VillageEvent::BuildingDowngraded {
            action_id: self.action_id,
            player_id: self.player_id,
            village_id: self.village_id,
            slot_id: self.slot_id,
            building_name: self.building_name.clone(),
            level: self.level,
            speed: self.speed,
        }])
    }
}
