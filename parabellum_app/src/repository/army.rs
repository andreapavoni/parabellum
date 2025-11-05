use uuid::Uuid;

use parabellum_core::ApplicationError;
use parabellum_game::models::army::Army;

#[async_trait::async_trait]
pub trait ArmyRepository: Send + Sync {
    async fn get_by_id(&self, army_id: Uuid) -> Result<Army, ApplicationError>;
    async fn save(&self, army: &Army) -> Result<(), ApplicationError>;
    async fn remove(&self, army_id: Uuid) -> Result<(), ApplicationError>;
}
