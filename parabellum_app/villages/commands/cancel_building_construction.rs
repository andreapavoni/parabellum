use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::common::ResourceGroup;
use parabellum_types::errors::{AppError, GameError};
use uuid::Uuid;

use crate::villages::{
    VillageAggregate, VillageEvent, commands::as_domain_error, commands::as_invariant_error,
};

#[derive(Debug, Clone)]
/// Cancels a queued building add, upgrade, or downgrade action.
pub struct CancelBuildingConstruction {
    pub action_ids: Vec<Uuid>,
    pub player_id: Uuid,
    pub village_id: u32,
    pub refund: ResourceGroup,
    pub canceled_at: DateTime<Utc>,
}

impl Command for CancelBuildingConstruction {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.village_id,
                actual: aggregate.aggregate_id(),
            }));
        }
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: self.village_id,
                player_id: self.player_id,
            }));
        }
        if self.action_ids.is_empty()
            || !self
                .action_ids
                .iter()
                .all(|action_id| aggregate.village().has_pending_building_action(*action_id))
        {
            return Err(as_domain_error(
                GameError::BuildingConstructionNotCancelable,
            ));
        }

        Ok(vec![VillageEvent::BuildingConstructionCanceled {
            action_ids: self.action_ids.clone(),
            player_id: self.player_id,
            village_id: self.village_id,
            refund: self.refund.clone(),
            canceled_at: self.canceled_at,
        }])
    }
}
