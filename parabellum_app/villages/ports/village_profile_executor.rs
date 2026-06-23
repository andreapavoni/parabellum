//! Village profile command execution gateway.
//!
//! The app use case builds village profile command intent and delegates
//! execution through infrastructure.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use crate::villages::RenameVillage;

/// Canonical village profile command intent produced by app use cases.
#[derive(Debug, Clone)]
pub enum VillageProfileCommandIntent {
    /// Rename an owned village.
    RenameVillage {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with rename data.
        command: RenameVillage,
    },
}

/// Executes village profile command intent through infrastructure.
#[async_trait]
pub trait VillageProfileCommandExecutor: Send + Sync {
    /// Persist and execute an already-planned village profile command intent.
    async fn execute_village_profile_command(
        &self,
        command: VillageProfileCommandIntent,
    ) -> Result<(), ApplicationError>;
}
