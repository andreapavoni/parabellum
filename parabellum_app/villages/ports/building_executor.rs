//! Building lifecycle command execution gateway.
//!
//! The app use case builds building command intent after applying app-level
//! settings, ownership checks, and cancellation-window checks.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use crate::villages::{
    AddBuilding, CancelBuildingConstruction, DowngradeBuilding, UpgradeBuilding,
};

/// Canonical building command intent produced by app use cases.
#[derive(Debug, Clone)]
pub enum BuildingCommandIntent {
    /// Schedule construction of a new building.
    AddBuilding {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with construction data.
        command: AddBuilding,
    },
    /// Schedule an existing building upgrade.
    UpgradeBuilding {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with upgrade data.
        command: UpgradeBuilding,
    },
    /// Schedule an existing building downgrade.
    DowngradeBuilding {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with downgrade data.
        command: DowngradeBuilding,
    },
    /// Cancel queued building construction actions.
    CancelBuildingConstruction {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with cancellation data.
        command: CancelBuildingConstruction,
    },
}

/// Executes building command intent through infrastructure.
#[async_trait]
pub trait BuildingCommandExecutor: Send + Sync {
    /// Persist and execute an already-planned building command intent.
    async fn execute_building_command(
        &self,
        command: BuildingCommandIntent,
    ) -> Result<(), ApplicationError>;
}
