//! Read/context port for village development use cases.
//!
//! Development use cases need village context for unit metadata and expansion
//! training validation before command intent is built.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use crate::{
    villages::models::VillageModel,
    villages::read_models::{VillageQueues, VillageTroopMovements},
};

/// Loads read-model context required by development use cases.
#[async_trait]
pub trait DevelopmentReadPort: Send + Sync {
    /// Returns the current village read model for unit metadata and hydration.
    async fn get_development_village(
        &self,
        village_id: u32,
    ) -> Result<VillageModel, ApplicationError>;

    /// Counts already-founded child villages for expansion-slot validation.
    async fn count_development_child_villages(
        &self,
        player_id: uuid::Uuid,
        village_id: u32,
    ) -> Result<u8, ApplicationError>;

    /// Returns current village queues for expansion-unit training commitments.
    async fn get_development_village_queues(
        &self,
        village_id: u32,
    ) -> Result<VillageQueues, ApplicationError>;

    /// Returns current troop movements for moving expansion-unit commitments.
    async fn get_development_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<VillageTroopMovements, ApplicationError>;
}
