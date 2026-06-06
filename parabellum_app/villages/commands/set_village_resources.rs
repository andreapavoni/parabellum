use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::{common::ResourceGroup, errors::GameError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

/// Utility command that sets village stored resources to requested amounts,
/// clamped by current warehouse/granary capacities.
///
/// If requested quantities exceed current capacities, the overflow is discarded.
#[derive(Debug, Clone)]
pub struct SetVillageResources {
    /// Command caller; must own the village.
    pub player_id: Uuid,
    /// Desired stored resources before capacity clamping.
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

        if aggregate.village().village.stored_resources() == self.resources {
            return Ok(vec![]);
        }

        Ok(vec![VillageEvent::VillageResourcesSet {
            player_id: self.player_id,
            village_id: aggregate.aggregate_id(),
            resources: self.resources.clone(),
        }])
    }
}
