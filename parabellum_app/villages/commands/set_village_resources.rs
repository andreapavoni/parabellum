use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::{common::ResourceGroup, errors::GameError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
pub struct SetVillageResources {
    pub player_id: Uuid,
    pub resources: ResourceGroup,
}

impl Command for SetVillageResources {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: aggregate.aggregate_id(),
                player_id: self.player_id,
            }));
        }

        Ok(vec![VillageEvent::VillageResourcesSet {
            player_id: self.player_id,
            village_id: aggregate.aggregate_id(),
            resources: self.resources.clone(),
        }])
    }
}
