//! Read/context port for hero use cases.
//!
//! Hero use cases need current hero state and player-level hero lifecycle
//! status before command intent is built.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parabellum_game::models::hero::Hero;
use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

/// Loads read-model context required by hero use cases.
#[async_trait]
pub trait HeroReadPort: Send + Sync {
    /// Returns a hero by id.
    async fn get_hero(&self, hero_id: Uuid) -> Result<Hero, ApplicationError>;

    /// Returns the player's current hero, if any.
    async fn get_hero_by_player(&self, player_id: Uuid) -> Result<Option<Hero>, ApplicationError>;

    /// Returns whether the player already has a living hero.
    async fn player_has_alive_hero(&self, player_id: Uuid) -> Result<bool, ApplicationError>;

    /// Returns whether the player already has a pending hero revival.
    async fn player_has_pending_hero_revival(
        &self,
        player_id: Uuid,
    ) -> Result<bool, ApplicationError>;

    /// Returns when the player's pending hero revival completes, if any.
    async fn get_pending_hero_revival_at(
        &self,
        player_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, ApplicationError>;
}
