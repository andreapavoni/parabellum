use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::buildings::BuildingName;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent};

#[derive(Debug, Clone)]
pub struct CompleteAddBuilding {
    pub action_id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub slot_id: u8,
    pub building_name: BuildingName,
    pub level: u8,
    pub speed: i8,
}

impl Command for CompleteAddBuilding {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.village_id {
            return Err(CqrsError::Domain("invalid aggregate target".to_string()));
        }
        Ok(vec![VillageEvent::BuildingAdded {
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
