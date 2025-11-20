use uuid::Uuid;

use parabellum_core::ApplicationError;
use parabellum_game::models::village::Village;

#[async_trait::async_trait]
pub trait VillageRepository: Send + Sync {
    async fn get_by_id(&self, village_id: u32) -> Result<Village, ApplicationError>;
    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Village>, ApplicationError>;
    async fn get_capital_by_player_id(&self, player_id: Uuid) -> Result<Village, ApplicationError>;
    async fn save(&self, village: &Village) -> Result<(), ApplicationError>;
}
