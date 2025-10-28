use anyhow::Result;
use uuid::Uuid;

use crate::game::models::army::Army;

#[async_trait::async_trait]
pub trait ArmyRepository: Send + Sync {
    async fn create(&self, army: &Army) -> Result<()>;
    async fn get_by_id(&self, army_id: Uuid) -> Result<Option<Army>>;
    async fn save(&self, army: &Army) -> Result<()>;
    async fn remove(&self, army_id: Uuid) -> Result<()>;
}
