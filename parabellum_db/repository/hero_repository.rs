use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::HeroRepository;
use parabellum_core::{ApplicationError, DbError, Result};
use parabellum_game::models::hero::Hero;

use crate::models as db_models;

#[derive(Clone)]
pub struct PostgresHeroRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresHeroRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> HeroRepository for PostgresHeroRepository<'a> {
    async fn save(&self, hero: &Hero) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO heroes (id, village_id, player_id, health, experience,
                                 attack_points, defense_points, off_bonus, def_bonus)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE
            SET
              village_id = $2,
              health = $4,
              experience = $5,
              attack_points = $6,
              defense_points = $7,
              off_bonus = $8,
              def_bonus = $9
            "#,
            hero.id,
            hero.village_id as i32,
            hero.player_id,
            hero.health as i16,
            hero.experience as i32,
            hero.attack_points as i32,
            hero.defense_points as i32,
            hero.off_bonus as i16,
            hero.def_bonus as i16,
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?; // wrap DB error
        Ok(())
    }

    async fn get_by_id(&self, hero_id: Uuid) -> Result<Hero, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let db_hero = sqlx::query_as!(
            db_models::Hero,
            r#"
            SELECT id, player_id, village_id, health, experience,
                   attack_points, defense_points, off_bonus, def_bonus
            FROM heroes
            WHERE id = $1
            "#,
            hero_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?; // returns error if not found
        Ok(db_hero.into())
    }
}
