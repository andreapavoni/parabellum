use crate::game::models::village::Village;
use anyhow::Result;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait VillageRepository: Send + Sync {
    async fn create(&self, village: &Village) -> Result<()>;
    async fn get_by_id(&self, village_id: u32) -> Result<Option<Village>>;
    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Village>>;
    async fn save(&self, village: &Village) -> Result<()>;
}
