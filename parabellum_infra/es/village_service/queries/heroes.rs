//! Hero read helpers for `VillageEsService`.
//!
//! These methods expose hero read-model state through the service facade. They
//! intentionally delegate storage details to projection repositories so hero
//! placement and lifecycle semantics stay owned by the app contracts and game
//! domain.

use std::sync::Arc;

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::projection_repositories::HeroRepository;

use crate::es::{PostgresHeroRepository, PostgresScheduledActionRepository};

use super::super::VillageEsService;

impl VillageEsService {
    /// Returns a projected hero by id.
    pub async fn get_hero(
        &self,
        hero_id: uuid::Uuid,
    ) -> Result<parabellum_game::models::hero::Hero, CqrsError> {
        let repo = PostgresHeroRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        repo.get_by_id(hero_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    /// Returns the projected hero owned by `player_id`, when one exists.
    pub async fn get_hero_by_player(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<Option<parabellum_game::models::hero::Hero>, CqrsError> {
        let repo = PostgresHeroRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        repo.get_by_player(player_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    /// Returns whether the player has a currently alive projected hero.
    pub async fn player_has_alive_hero(&self, player_id: uuid::Uuid) -> Result<bool, CqrsError> {
        let repo: Arc<dyn HeroRepository> = Arc::new(PostgresHeroRepository::new(
            crate::ProjectionDb::new(self.pool.clone()),
        ));
        repo.has_alive_for_player(player_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    /// Returns the earliest pending hero revival time for the player.
    pub async fn pending_hero_revival_at(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, CqrsError> {
        PostgresScheduledActionRepository::new(crate::ProjectionDb::new(self.pool.clone()))
            .pending_hero_revival_for_player(player_id)
            .await
            .map(|action| action.map(|action| action.execute_at))
            .map_err(CqrsError::domain_source)
    }
}
