use std::collections::HashMap;
use uuid::Uuid;

use parabellum_game::models::village::Village;
use parabellum_types::{errors::ApplicationError, map::Position};

/// Minimal village info for display purposes (name and position)
#[derive(Debug, Clone, PartialEq)]
pub struct VillageInfo {
    pub id: u32,
    pub name: String,
    pub position: Position,
}

#[async_trait::async_trait]
pub trait VillageRepository: Send + Sync {
    async fn get_by_id(&self, village_id: u32) -> Result<Village, ApplicationError>;
    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Village>, ApplicationError>;
    async fn save(&self, village: &Village) -> Result<(), ApplicationError>;

    /// Fetch basic info (name, position) for multiple villages by IDs
    async fn get_info_by_ids(
        &self,
        village_ids: &[u32],
    ) -> Result<HashMap<u32, VillageInfo>, ApplicationError>;
}
