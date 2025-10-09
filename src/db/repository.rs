use std::env;

use anyhow::Result;
use ormlite::{sqlite::SqlitePoolOptions, Model, Pool};
use sqlx::{pool::PoolConnection, Sqlite, SqlitePool};

use super::models::{map::MapField, village::Village};
use crate::game::models::{
    map::{generate_new_map, Oasis, Valley},
    village::Village as GameVillage,
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

    pub fn with_poolection(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_pool_connection(&self) -> Result<PoolConnection<Sqlite>> {
        let conn = self.pool.acquire().await?;
        Ok(conn)
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
        let mut conn = self.pool.acquire().await?;

        println!("Generating a map of {} fields", size * size * 4);
        for f in map {
            let fm: MapField = f.into();
            fm.insert(&mut conn).await?;
        }
        println!("Map generated");

        Ok(())
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
