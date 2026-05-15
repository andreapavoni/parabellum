use async_trait::async_trait;
use parabellum_game::models::map::MapQuadrant;
use parabellum_types::common::{Player, User};
use parabellum_types::errors::ApplicationError;
use parabellum_types::tribe::Tribe;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RegisterPlayerRequest {
    pub player_id: Uuid,
    pub username: String,
    pub email: String,
    pub password: String,
    pub tribe: Tribe,
    pub quadrant: MapQuadrant,
}

#[async_trait]
pub trait IdentityPort: Send + Sync {
    async fn register_player(&self, request: RegisterPlayerRequest)
    -> Result<(), ApplicationError>;
    async fn authenticate_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<User, ApplicationError>;
    async fn get_user_by_email(&self, email: &str) -> Result<User, ApplicationError>;
    async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, ApplicationError>;
    async fn get_player_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError>;
    async fn get_player_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError>;
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Saves a user.
    async fn save(&self, email: String, password_hash: String) -> Result<(), ApplicationError>;

    /// Find user by email.
    async fn get_by_email(&self, email: &str) -> Result<User, ApplicationError>;

    /// Find user by id.
    async fn get_by_id(&self, user_id: Uuid) -> Result<User, ApplicationError>;
}

#[async_trait]
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
    ) -> Result<(Vec<crate::query_models::PlayerLeaderboardEntry>, i64), ApplicationError>;

    /// Updates player's total culture points by aggregating from all their villages.
    async fn update_culture_points(&self, player_id: Uuid) -> Result<(), ApplicationError>;

    /// Gets the total culture points production (CPP) per day for all player's villages.
    async fn get_total_culture_points_production(
        &self,
        player_id: Uuid,
    ) -> Result<u32, ApplicationError>;
}
