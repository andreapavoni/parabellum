use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
pub struct RenameVillage {
    pub player_id: Uuid,
    pub village_name: String,
}

impl Command for RenameVillage {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        let village_id = aggregate.aggregate_id();
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id,
                player_id: self.player_id,
            }));
        }

        let name = self.village_name.trim();
        if name.is_empty() || name.len() > 32 {
            return Err(as_domain_error(GameError::InvalidVillageName));
        }

        if aggregate.village().village.name == name {
            return Ok(vec![]);
        }

        Ok(vec![VillageEvent::VillageRenamed {
            village_id,
            player_id: self.player_id,
            village_name: name.to_string(),
        }])
    }
}
