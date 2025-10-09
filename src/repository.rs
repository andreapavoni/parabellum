use anyhow::Result;
use uuid::Uuid;

use crate::game::models::{
    map::{Oasis, Quadrant, Valley},
    village::Village,
    Player, Tribe,
};

#[async_trait::async_trait]
pub trait Repository: Send + Sync {
    async fn bootstrap_new_map(&self, size: u32) -> Result<()>;
    async fn register_player(&self, username: String, tribe: Tribe) -> Result<Player>;
    async fn get_unoccupied_valley(&self, quadrant: Option<Quadrant>) -> Result<Valley>;
    async fn get_player_by_id(&self, player_id: Uuid) -> Result<Player>;
    async fn get_player_by_username(&self, username: String) -> Result<Player>;
    async fn get_village_by_id(&self, village_id: u32) -> Result<Village>;
    async fn get_valley_by_id(&self, valley_id: u32) -> Result<Valley>;
    async fn get_oasis_by_id(&self, oasis_id: u32) -> Result<Oasis>;
}
