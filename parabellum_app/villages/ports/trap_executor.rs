//! Trap command/workflow execution gateway.
//!
//! The app use case builds trap command intent after validating ownership,
//! resources, capacity, and timing.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use crate::villages::BuildTraps;

/// Canonical trap command intent produced by app use cases.
#[derive(Debug, Clone)]
pub enum TrapCommandIntent {
    /// Queue trap construction for a village.
    BuildTraps {
        /// Aggregate id for the village with the trapper.
        village_id: u32,
        /// Domain command with planned trapper state and execution time.
        command: BuildTraps,
    },
}

/// Executes trap command intent through infrastructure.
#[async_trait]
pub trait TrapCommandExecutor: Send + Sync {
    /// Persist and execute the already-planned trap command intent.
    async fn execute_trap_command(
        &self,
        command: TrapCommandIntent,
    ) -> Result<(), ApplicationError>;
}
