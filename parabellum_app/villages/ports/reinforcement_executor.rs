//! Reinforcement command/workflow execution gateway.
//!
//! The app use case builds reinforcement and trapped-troop command intent.
//! Infrastructure implements this port with CQRS/ES command execution and
//! workflow event appends.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use crate::villages::{
    DisbandTrappedTroops, RecallReinforcements, ReleaseReinforcements, ReleaseTrappedTroops,
};

/// Canonical reinforcement/trapped-troop command intent produced by app use cases.
#[derive(Debug, Clone)]
pub enum ReinforcementCommandIntent {
    /// Recall selected units from a stationed reinforcement back to their home village.
    RecallReinforcements {
        /// Aggregate id for the home village.
        home_village_id: u32,
        /// Domain command with selected units and planned return.
        command: RecallReinforcements,
    },
    /// Release selected units stationed in a village back to their home village.
    ReleaseReinforcements {
        /// Aggregate id for the stationed village.
        stationed_village_id: u32,
        /// Domain command with selected units and planned return.
        command: ReleaseReinforcements,
    },
    /// Release a trapped army from the village holding it.
    ReleaseTrappedTroops {
        /// Aggregate id for the village holding the trapped army.
        trapped_village_id: u32,
        /// Domain command with trapper state and planned return.
        command: ReleaseTrappedTroops,
    },
    /// Disband a trapped army owned by the requesting player.
    DisbandTrappedTroops {
        /// Aggregate id for the village holding the trapped army.
        trapped_village_id: u32,
        /// Domain command with trapper state after release.
        command: DisbandTrappedTroops,
    },
}

/// Executes reinforcement command intent through infrastructure.
#[async_trait]
pub trait ReinforcementCommandExecutor: Send + Sync {
    /// Persist and execute the already-planned reinforcement command intent.
    async fn execute_reinforcement_command(
        &self,
        command: ReinforcementCommandIntent,
    ) -> Result<(), ApplicationError>;
}
