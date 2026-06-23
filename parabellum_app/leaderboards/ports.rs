use async_trait::async_trait;

use parabellum_types::errors::ApplicationError;

use crate::read_models::PlayerPopulationLeaderboardEntry;

/// Read port for leaderboard projections.
#[async_trait]
pub trait LeaderboardReadPort: Send + Sync {
    /// Returns player population leaderboard rows and the total player count.
    async fn list_player_population_entries(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<PlayerPopulationLeaderboardEntry>, i64), ApplicationError>;
}
