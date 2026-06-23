//! Village command execution gateway.
//!
//! Application use cases build canonical command intent and delegate execution
//! through this port. Infrastructure implements the port with the CQRS/ES
//! runtime and maps runtime failures into `ApplicationError`.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use crate::villages::{AttackVillage, ScoutVillage, SendReinforcement, SendSettlers};

/// Canonical outbound movement command intent produced by movement use cases.
#[derive(Debug, Clone)]
pub enum VillageCommandIntent {
    /// Send selected troops to support another village.
    SendReinforcement(SendReinforcement),
    /// Send selected troops to attack or raid another village.
    AttackVillage(AttackVillage),
    /// Send scouts to inspect another village.
    ScoutVillage(ScoutVillage),
    /// Send settlers to found a village on an unoccupied valley.
    SendSettlers(SendSettlers),
}

/// Executes already-planned village command intent through infrastructure.
#[async_trait]
pub trait VillageCommandExecutor: Send + Sync {
    /// Persist and execute a command against the given village aggregate.
    async fn execute_village_command(
        &self,
        village_id: u32,
        command: VillageCommandIntent,
    ) -> Result<(), ApplicationError>;
}
