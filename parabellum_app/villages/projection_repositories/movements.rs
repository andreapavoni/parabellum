//! Village movement projection repository contracts.

use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::models::{MovementDirection, MovementType, VillageMovement};

/// Village movement query filter.
///
/// Movement rows are stored from one viewing village perspective, so every
/// query is anchored to a single village id. Optional predicates narrow the
/// visible movement rows without exposing database columns to callers.
#[derive(Debug, Clone)]
pub struct VillageMovementFilter {
    /// Viewing village whose movement rows are requested.
    pub village_id: u32,
    /// Optional direction restrictions from the viewing village perspective.
    pub directions: Vec<MovementDirection>,
    /// Optional movement category restrictions.
    pub movement_types: Vec<MovementType>,
}

impl VillageMovementFilter {
    /// Creates a filter for all movement rows visible to `village_id`.
    pub fn for_village(village_id: u32) -> Self {
        Self {
            village_id,
            directions: Vec::new(),
            movement_types: Vec::new(),
        }
    }

    /// Restricts the filter to one direction.
    pub fn direction(mut self, direction: MovementDirection) -> Self {
        self.directions = vec![direction];
        self
    }

    /// Restricts the filter to any of the given directions.
    pub fn directions(mut self, directions: impl IntoIterator<Item = MovementDirection>) -> Self {
        self.directions = directions.into_iter().collect();
        self
    }

    /// Restricts the filter to one movement category.
    pub fn movement_type(mut self, movement_type: MovementType) -> Self {
        self.movement_types = vec![movement_type];
        self
    }

    /// Restricts the filter to any of the given movement categories.
    pub fn movement_types(
        mut self,
        movement_types: impl IntoIterator<Item = MovementType>,
    ) -> Self {
        self.movement_types = movement_types.into_iter().collect();
        self
    }
}

/// Persistence boundary for projected troop movement rows.
#[async_trait::async_trait]
pub trait VillageMovementRepository: Send + Sync {
    async fn upsert(&self, movement: &VillageMovement) -> Result<(), ApplicationError>;

    async fn list_movements(
        &self,
        filter: VillageMovementFilter,
    ) -> Result<Vec<VillageMovement>, ApplicationError>;

    async fn delete_by_movement_id(&self, movement_id: Uuid) -> Result<(), ApplicationError>;
}
