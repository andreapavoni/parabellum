use crate::game::models::{Player, Tribe};
use anyhow::Result;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait PlayerRepository: Send + Sync {
    async fn create(&self, id: Uuid, username: String, tribe: Tribe) -> Result<Player>;
    async fn get_by_id(&self, player_id: Uuid) -> Result<Option<Player>>;
    async fn get_by_username(&self, username: &str) -> Result<Option<Player>>;
    // async fn save(&self, player: &Player) -> Result<()>; // Per gli aggiornamenti
}
