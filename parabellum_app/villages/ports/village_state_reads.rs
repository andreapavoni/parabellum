use async_trait::async_trait;
use uuid::Uuid;

use parabellum_types::errors::ApplicationError;

use crate::villages::models::VillageModel;

/// Read port for full village projection state.
#[async_trait]
pub trait VillageStateReadPort: Send + Sync {
    /// Loads one full village projection state.
    async fn get_village_state(&self, village_id: u32) -> Result<VillageModel, ApplicationError>;

    /// Lists full village projection states owned by one player.
    async fn list_player_village_states(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<VillageModel>, ApplicationError>;
}
