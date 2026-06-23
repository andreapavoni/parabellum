//! Read/context port for village army views.
//!
//! Army view reads are app-facing summaries of home, deployed, reinforced, and
//! trapped armies.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use crate::villages::read_models::VillageArmyStateView;

/// Loads app-facing village army views.
#[async_trait]
pub trait VillageArmyReadPort: Send + Sync {
    /// Returns the full army state view for a village.
    async fn get_village_army_state_view(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyStateView, ApplicationError>;
}
