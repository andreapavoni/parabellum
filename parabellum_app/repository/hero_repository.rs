use uuid::Uuid;

use parabellum_game::models::hero::Hero;
use parabellum_types::errors::{ApplicationError, Result};

#[async_trait::async_trait]
pub trait HeroRepository: Send + Sync {
    /// Creates a new hero in the database
    async fn save(&self, hero: &Hero) -> Result<(), ApplicationError>;

    /// Retrieves a hero by its UUID
    async fn get_by_id(&self, hero_id: Uuid) -> Result<Hero, ApplicationError>;
}
