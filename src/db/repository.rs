use std::env;

use anyhow::{Error, Result};
use ormlite::{sqlite::SqlitePoolOptions, types::Json, Model, Pool};
use sqlx::{pool::PoolConnection, Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use super::models::{map::MapField, player::Player, village::Village};
use crate::game::models::{
    map::{generate_new_map, Oasis, Quadrant, Valley},
    village::Village as GameVillage,
    Player as GamePlayer, Tribe,
};

// use crate::game::models::village::Village;

#[derive(Debug, Clone)]
pub struct Repository {
    pool: SqlitePool,
}

impl Repository {
    pub async fn new_from_env() -> Result<Self> {
        let url = env::var("DATABASE_URL").expect("DATABASE_URL is not set");
        let pool = Self::new_connection_pool(&url).await?;
        Ok(Self { pool })
    }

    pub async fn new(url: String) -> Result<Self> {
        let pool = Self::new_connection_pool(&url).await?;
        Ok(Self { pool })
    }

    pub fn with_connection_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_pool_connection(&self) -> Result<PoolConnection<Sqlite>> {
        let conn = self.pool.acquire().await?;
        Ok(conn)
    }

    pub async fn begin_transaction(&self) -> Result<Transaction<Sqlite>> {
        let tx = self.pool.begin().await?;
        Ok(tx)
    }

    async fn new_connection_pool(url: &str) -> Result<Pool<Sqlite>> {
        Ok(SqlitePoolOptions::new()
            .max_connections(20)
            .connect(&url)
            .await?)
    }
}
#[async_trait::async_trait]
impl crate::repository::Repository for Repository {
    async fn bootstrap_new_map(&self, size: u32) -> Result<()> {
        let map = generate_new_map(size as i32);
        let mut tx = self.begin_transaction().await?;

        print!("Generating a map of {} fields... ", size * size * 4);
        for f in map {
            let fm: MapField = f.into();
            fm.insert(&mut tx).await?;
        }
        tx.commit().await?;
        println!("done!");

        Ok(())
    }

    async fn get_unoccupied_valley(&self, quadrant: Option<Quadrant>) -> Result<Valley> {
        let mut conn = self.get_pool_connection().await?;
        let query = match quadrant {
            Some(Quadrant::NorthEast) => {
                MapField::query("SELECT * FROM map_fields WHERE player_id IS NULL AND village_id IS NULL AND x >= 0 AND y >= 0 AND topology = '{\"Valley\":[4,4,4,6]}' ORDER BY
	RANDOM()")
            }
            Some(Quadrant::EastSouth) => {
                MapField::query("SELECT * FROM map_fields WHERE player_id IS NULL AND village_id IS NULL AND x >= 0 AND y < 0 AND topology = '{\"Valley\":[4,4,4,6]}' ORDER BY
	RANDOM()" )
            }
            Some(Quadrant::SouthWest) => {
                MapField::query("SELECT * FROM map_fields WHERE player_id IS NULL AND village_id IS NULL AND x < 0 AND y < 0 AND topology = '{\"Valley\":[4,4,4,6]}' ORDER BY
	RANDOM()")
            }
            Some(Quadrant::WestNorth) => {
                MapField::query("SELECT * FROM map_fields WHERE player_id IS NULL AND village_id IS NULL AND x >= 0 AND y < 0 AND topology = '{\"Valley\":[4,4,4,6]}' ORDER BY
	RANDOM()")
            }
            None => { MapField::query("SELECT * FROM map_fields WHERE player_id IS NULL AND village_id IS NULL AND topology = '{\"Valley\":[4,4,4,6]}' ORDER BY RANDOM()") }
        };
        let valley = query.fetch_one(&mut conn).await?;

        Ok(valley.try_into()?)
    }

    async fn register_player(&self, username: String, tribe: Tribe) -> Result<GamePlayer> {
        let mut tx = self.begin_transaction().await?;

        if let Ok(_) = Player::query("SELECT * FROM players WHERE username = ?")
            .bind(username.clone())
            // FIXME this method is better to lookup records by their columns `.fetch_optional(&mut tx)`
            .fetch_one(&mut tx)
            .await
        {
            return Err(Error::msg("Username already used."));
        }

        let player = Player {
            id: Uuid::new_v4(),
            username,
            tribe: Json(tribe),
        };
        player.clone().insert(&mut tx).await?;

        tx.commit().await?;

        Ok(player.into())
    }

    async fn get_player_by_id(&self, player_id: Uuid) -> Result<GamePlayer> {
        let mut conn = self.get_pool_connection().await?;
        let player = Player::query("SELECT * FROM players WHERE id = ?")
            .bind(player_id)
            .fetch_one(&mut conn)
            .await?;

        Ok(player.into())
    }

    async fn get_player_by_username(&self, username: String) -> Result<GamePlayer> {
        let mut conn = self.get_pool_connection().await?;
        let player = Player::query("SELECT * FROM players WHERE username = ?")
            .bind(username)
            .fetch_one(&mut conn)
            .await?;

        Ok(player.into())
    }

    async fn get_village_by_id(&self, village_id: u32) -> Result<GameVillage> {
        let mut conn = self.get_pool_connection().await?;
        let village = Village::query("SELECT * FROM villages WHERE id = ?")
            .bind(village_id)
            .fetch_one(&mut conn)
            .await?;

        Ok(village.into())
    }

    async fn get_valley_by_id(&self, valley_id: u32) -> Result<Valley> {
        let mut conn = self.get_pool_connection().await?;
        let valley = MapField::query("SELECT * FROM map_fields WHERE id = ?")
            .bind(valley_id)
            .fetch_one(&mut conn)
            .await?;

        Ok(valley.try_into()?)
    }

    async fn get_oasis_by_id(&self, oasis_id: u32) -> Result<Oasis> {
        let mut conn = self.get_pool_connection().await?;
        let oasis = MapField::query("SELECT * FROM map_fields WHERE id = ?")
            .bind(oasis_id)
            .fetch_one(&mut conn)
            .await?;

        Ok(oasis.try_into()?)
    }
}
