//! Movement-control command/workflow execution gateway.
//!
//! The app use case builds movement-control command intent after validating
//! source ownership and the cancel window.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use crate::villages::CancelTroopMovement;

/// Canonical movement-control command intent produced by app use cases.
#[derive(Debug, Clone)]
pub enum MovementControlCommandIntent {
    /// Cancel an outgoing troop movement and schedule the army return.
    CancelTroopMovement {
        /// Aggregate id for the source village.
        source_village_id: u32,
        /// Domain command with cancel/return workflow data.
        command: CancelTroopMovement,
    },
}

/// Executes movement-control command intent through infrastructure.
#[async_trait]
pub trait MovementControlCommandExecutor: Send + Sync {
    /// Persist and execute the already-planned movement-control command intent.
    async fn execute_movement_control_command(
        &self,
        command: MovementControlCommandIntent,
    ) -> Result<(), ApplicationError>;
}
