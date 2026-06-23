//! Read/context port for movement dispatch use cases.
//!
//! The movement use case needs authoritative village, hero, and map occupancy
//! context before it can build outbound movement command intent. Infrastructure
//! implements these reads from current read models.

use async_trait::async_trait;
use parabellum_game::models::hero::Hero;
use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::models::VillageModel;

/// Loads read-model context required by movement dispatch use cases.
#[async_trait]
pub trait MovementReadPort: Send + Sync {
    /// Returns the current village read model for ownership and travel planning.
    async fn get_movement_village(&self, village_id: u32)
    -> Result<VillageModel, ApplicationError>;

    /// Returns the hero selected for movement dispatch.
    async fn get_movement_hero(&self, hero_id: Uuid) -> Result<Hero, ApplicationError>;

    /// Returns whether the map field can receive a settler founding movement.
    async fn is_unoccupied_valley(&self, field_id: u32) -> Result<bool, ApplicationError>;
}
