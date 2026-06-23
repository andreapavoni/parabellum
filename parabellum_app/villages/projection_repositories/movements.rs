//! Village movement projection repository contracts.

use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::models::VillageMovement;

/// Persistence boundary for projected troop movement rows.
#[async_trait::async_trait]
pub trait VillageMovementRepository: Send + Sync {
    async fn upsert(&self, movement: &VillageMovement) -> Result<(), ApplicationError>;

    async fn list_by_village_id(
        &self,
        village_id: u32,
    ) -> Result<Vec<VillageMovement>, ApplicationError>;

    async fn delete_by_movement_id(&self, movement_id: Uuid) -> Result<(), ApplicationError>;
}
