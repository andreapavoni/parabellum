use uuid::Uuid;

use crate::{Result, error::ApplicationError, game::models::Player};

#[async_trait::async_trait]
pub trait PlayerRepository: Send + Sync {
    async fn create(&self, player: &Player) -> Result<(), ApplicationError>;
    async fn get_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError>;
    // async fn save(&self, player: &Player) -> Result<()>;
}
