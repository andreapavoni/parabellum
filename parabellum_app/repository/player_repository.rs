use parabellum_core::ApplicationError;
use parabellum_game::models::player::Player;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait PlayerRepository: Send + Sync {
    /// Saves a player (creates if new, updates if exists).
    async fn save(&self, player: &Player) -> Result<(), ApplicationError>;

    /// Returns a player by id.
    async fn get_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError>;

    /// Returns a player by user id.
    async fn get_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError>;
}
