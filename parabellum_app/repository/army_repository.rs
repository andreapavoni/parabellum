use uuid::Uuid;

use parabellum_game::models::army::Army;
use parabellum_types::errors::ApplicationError;

#[async_trait::async_trait]
pub trait ArmyRepository: Send + Sync {
    /// Returns army by its ID.
    async fn get_by_id(&self, army_id: Uuid) -> Result<Army, ApplicationError>;

    /// Returns army by hero ID.
    async fn get_by_hero_id(&self, hero_id: Uuid) -> Result<Army, ApplicationError>;

    async fn set_hero(&self, army_id: Uuid, hero_id: Option<Uuid>) -> Result<(), ApplicationError>;

    async fn save(&self, army: &Army) -> Result<(), ApplicationError>;
    async fn remove(&self, army_id: Uuid) -> Result<(), ApplicationError>;
}
