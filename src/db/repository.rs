use anyhow::Result;
use uuid::Uuid;

use crate::{
    db::DbPool,
    game::models::{
        map::{MapField, MapQuadrant, Position, Valley},
        village::Village,
        Player, Tribe,
    },
    repository::*,
};

pub struct PostgresRepository {
    pool: DbPool, // Il tuo pool di connessioni Diesel
}

impl PostgresRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl PlayerRepository for PostgresRepository {
    async fn create(&self, username: String, tribe: Tribe) -> Result<Player> {}
    async fn get_by_id(&self, player_id: Uuid) -> Result<Option<Player>> {}
    async fn get_by_username(&self, username: &str) -> Result<Option<Player>> {}
}

#[async_trait::async_trait]
impl VillageRepository for PostgresRepository {
    async fn create(&self, village: &Village) -> Result<()> {}
    async fn get_by_id(&self, village_id: u32) -> Result<Option<Village>> {}
    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Village>> {}
    async fn save(&self, village: &Village) -> Result<()> {}
}

#[async_trait::async_trait]
impl MapRepository for PostgresRepository {
    async fn find_unoccupied_valley(&self, quadrant: &MapQuadrant) -> Result<Valley> {}
    async fn get_field_by_id(&self, id: i32) -> Result<Option<MapField>> {}
}

#[async_trait::async_trait]
impl ArmyRepository for PostgresRepository {}

#[async_trait::async_trait]
impl JobRepository for PostgresRepository {}
