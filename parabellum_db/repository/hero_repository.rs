use sqlx::{Postgres, Transaction, types::Json};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::HeroRepository;
use parabellum_game::models::hero::Hero;
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
};

use crate::models::{self as db_models, Tribe};

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
        let db_tribe: Tribe = hero.tribe.clone().into();

        sqlx::query!(
            r#"
            INSERT INTO heroes (id, village_id, player_id, tribe, level, health, experience,
                                 strength_points, regeneration_points, off_bonus_points, def_bonus_points, resources_points, unassigned_points, resource_focus)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (id) DO UPDATE
            SET
              village_id = $2,
              level = $5,
              health = $6,
              experience = $7,
              strength_points = $8,
              regeneration_points = $9,
              off_bonus_points = $10,
              def_bonus_points = $11,
              resources_points = $12,
              unassigned_points = $13,
              resource_focus = $14
            "#,
            hero.id,
            hero.village_id as i32,
            hero.player_id,
            db_tribe as _,
            hero.level as i16,
            hero.health as i16,
            hero.experience as i32,
            hero.strength_points as i16,
            hero.regeneration_points as i16,
            hero.off_bonus_points as i16,
            hero.def_bonus_points as i16,
            hero.resources_points as i16,
            hero.unassigned_points as i16,
            Json(&hero.resource_focus) as _,
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
            SELECT id, player_id, village_id, tribe as "tribe: _", level, health, experience, resource_focus,
                   strength_points, regeneration_points, off_bonus_points, def_bonus_points, resources_points, unassigned_points
            FROM heroes
            WHERE id = $1
            "#,
            hero_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::HeroNotFound(hero_id)))?; // returns error if not found
        Ok(db_hero.into())
    }
}
