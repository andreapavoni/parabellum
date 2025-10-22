use crate::game::models::Player;
use anyhow::Result;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait PlayerRepository: Send + Sync {
    async fn create(&self, player: &Player) -> Result<()>;
    async fn get_by_id(&self, player_id: Uuid) -> Result<Option<Player>>;
    async fn get_by_username(&self, username: &str) -> Result<Option<Player>>;
    // async fn save(&self, player: &Player) -> Result<()>; // Per gli aggiornamenti
}
