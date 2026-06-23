use async_trait::async_trait;
use uuid::Uuid;

use parabellum_types::{common::Player, errors::ApplicationError};

use crate::villages::projection_repositories::ExpansionCultureSnapshot;

/// Read port for village expansion culture information.
#[async_trait]
pub trait ExpansionReadPort: Send + Sync {
    /// Loads culture-point production and village-count data for expansion.
    async fn get_expansion_culture_snapshot(
        &self,
        player_id: Uuid,
        village_id: u32,
    ) -> Result<ExpansionCultureSnapshot, ApplicationError>;

    /// Advances the player's stored culture points before returning expansion data.
    async fn refresh_player_culture_points(&self, player_id: Uuid) -> Result<(), ApplicationError>;

    /// Loads the player after culture-point refresh.
    async fn get_player(&self, player_id: Uuid) -> Result<Player, ApplicationError>;
}
