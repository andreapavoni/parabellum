//! Village development command execution gateway.
//!
//! The app use case builds training/research command intent after applying app
//! settings and expansion-training validation.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use crate::villages::{ResearchAcademy, ResearchSmithy, TrainUnits};

/// Canonical development command intent produced by app use cases.
#[derive(Debug, Clone)]
pub enum DevelopmentCommandIntent {
    /// Queue unit training.
    TrainUnits {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with training data.
        command: TrainUnits,
    },
    /// Queue academy research.
    ResearchAcademy {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with academy research data.
        command: ResearchAcademy,
    },
    /// Queue smithy research.
    ResearchSmithy {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with smithy research data.
        command: ResearchSmithy,
    },
}

/// Executes development command intent through infrastructure.
#[async_trait]
pub trait DevelopmentCommandExecutor: Send + Sync {
    /// Persist and execute an already-planned development command intent.
    async fn execute_development_command(
        &self,
        command: DevelopmentCommandIntent,
    ) -> Result<(), ApplicationError>;
}
