//! Read/context port for trap-building use cases.
//!
//! Trap use cases require current village state and trapped-army occupancy to
//! plan buildable quantities through the domain trapper helper.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use crate::{villages::models::VillageModel, villages::read_models::VillageArmyStateView};

/// Loads read-model context required by trap use cases.
#[async_trait]
pub trait TrapReadPort: Send + Sync {
    /// Returns the current village read model for ownership, resources, and trapper state.
    async fn get_trap_village(&self, village_id: u32) -> Result<VillageModel, ApplicationError>;

    /// Returns army occupancy for trap-capacity calculations.
    async fn get_trap_army_state(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyStateView, ApplicationError>;
}
