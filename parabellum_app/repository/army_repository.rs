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

    /// Save army if it has units or hero, otherwise remove it
    async fn save_or_remove(&self, army: &Army) -> Result<(), ApplicationError> {
        if army.immensity() == 0 {
            self.remove(army.id).await
        } else {
            self.save(army).await
        }
    }
}
