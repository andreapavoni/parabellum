use parabellum_types::common::Player;
use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::query_models::PlayerLeaderboardEntry;

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

    /// Updates player's total culture points by aggregating from all their villages.
    async fn update_culture_points(&self, player_id: Uuid) -> Result<(), ApplicationError>;

    /// Gets the total culture points production (CPP) per day for all player's villages.
    async fn get_total_culture_points_production(
        &self,
        player_id: Uuid,
    ) -> Result<u32, ApplicationError>;
}
