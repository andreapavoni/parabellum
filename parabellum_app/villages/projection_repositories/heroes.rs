//! Hero projection repository contracts.

use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

/// Persistence boundary for projected heroes.
#[async_trait::async_trait]
pub trait HeroRepository: Send + Sync {
    async fn upsert(
        &self,
        hero: &parabellum_game::models::hero::Hero,
        home_village_id: u32,
        current_village_id: u32,
        state: &str,
    ) -> Result<(), ApplicationError>;

    async fn get_by_id(
        &self,
        hero_id: Uuid,
    ) -> Result<parabellum_game::models::hero::Hero, ApplicationError>;

    async fn get_by_player(
        &self,
        player_id: Uuid,
    ) -> Result<Option<parabellum_game::models::hero::Hero>, ApplicationError>;

    async fn has_alive_for_player(&self, player_id: Uuid) -> Result<bool, ApplicationError>;
}
