use uuid::Uuid;

use parabellum_types::common::Player;
use parabellum_types::errors::ApplicationError;

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerLeaderboardEntry {
    pub player_id: Uuid,
    pub username: String,
    pub village_count: i64,
    pub population: i64,
}

#[async_trait::async_trait]
pub trait PlayerRepository: Send + Sync {
    /// Saves a player (creates if new, updates if exists).
    async fn save(&self, player: &Player) -> Result<(), ApplicationError>;

    /// Returns a player by id.
    async fn get_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError>;

    /// Returns a player by user id.
    async fn get_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError>;

    /// Returns a paginated leaderboard ordered by total population (sum of all player villages).
    /// Also returns total player count for pagination purposes.
    async fn leaderboard_page(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<PlayerLeaderboardEntry>, i64), ApplicationError>;
}
