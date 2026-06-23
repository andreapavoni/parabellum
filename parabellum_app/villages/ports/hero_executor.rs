//! Hero command execution gateway.
//!
//! The app use case builds hero command intent after loading hero lifecycle
//! context and applying runtime settings.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use crate::villages::{
    AssignHeroPoints, CreateHero, ResetHeroPoints, ReviveHero, SetHeroResourceFocus,
};

/// Canonical hero command intent produced by app use cases.
#[derive(Debug, Clone)]
pub enum HeroCommandIntent {
    /// Create a hero in a village.
    CreateHero {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with hero creation data.
        command: CreateHero,
    },
    /// Queue hero revival.
    ReviveHero {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with hero revival data.
        command: ReviveHero,
    },
    /// Assign hero points.
    AssignHeroPoints {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with point assignment data.
        command: AssignHeroPoints,
    },
    /// Reset hero points.
    ResetHeroPoints {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with point reset data.
        command: ResetHeroPoints,
    },
    /// Change hero resource focus.
    SetHeroResourceFocus {
        /// Aggregate id for the village.
        village_id: u32,
        /// Domain command with resource focus data.
        command: SetHeroResourceFocus,
    },
}

/// Executes hero command intent through infrastructure.
#[async_trait]
pub trait HeroCommandExecutor: Send + Sync {
    /// Persist and execute an already-planned hero command intent.
    async fn execute_hero_command(
        &self,
        command: HeroCommandIntent,
    ) -> Result<(), ApplicationError>;
}
