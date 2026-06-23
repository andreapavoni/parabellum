//! Read/context port for reinforcement and trapped-troop control use cases.
//!
//! Reinforcement control use cases need authoritative village, army placement,
//! and trap occupancy context before building command intent.

use async_trait::async_trait;
use parabellum_game::models::army::Army;
use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::{villages::models::VillageModel, villages::read_models::VillageArmyStateView};

/// Current location and home ownership context for a stationed reinforcement.
#[derive(Debug, Clone)]
pub struct ReinforcementArmyContext {
    /// Village where the reinforcement is currently stationed.
    pub stationed_village_id: u32,
    /// Home village that owns the army.
    pub home_village_id: u32,
    /// Stationed army state.
    pub army: Army,
}

/// Current location and home ownership context for a trapped army.
#[derive(Debug, Clone)]
pub struct TrappedArmyContext {
    /// Village where the army is trapped.
    pub trapped_village_id: u32,
    /// Home village that owns the army.
    pub home_village_id: u32,
    /// Trapped army state.
    pub army: Army,
}

/// Loads read-model context required by reinforcement control use cases.
#[async_trait]
pub trait ReinforcementReadPort: Send + Sync {
    /// Returns the current stationed reinforcement context for an army id.
    async fn get_reinforcement_context(
        &self,
        army_id: Uuid,
    ) -> Result<ReinforcementArmyContext, ApplicationError>;

    /// Returns the current trapped army context for an army id.
    async fn get_trapped_army_context(
        &self,
        army_id: Uuid,
    ) -> Result<TrappedArmyContext, ApplicationError>;

    /// Returns a village read model for ownership, trapper, and travel planning.
    async fn get_reinforcement_village(
        &self,
        village_id: u32,
    ) -> Result<VillageModel, ApplicationError>;

    /// Returns army occupancy for trap-capacity calculations.
    async fn get_reinforcement_army_state(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyStateView, ApplicationError>;
}
