use sqlx::{Postgres, Transaction, types::Json};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::ArmyRepository;
use parabellum_game::models::army::Army;
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
};

use crate::models::{self as db_models, Tribe};

#[derive(Clone)]
pub struct PostgresArmyRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresArmyRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> ArmyRepository for PostgresArmyRepository<'a> {
    async fn get_by_id(&self, army_id: Uuid) -> Result<Army, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let army = sqlx::query_as!(
            db_models::Army,
            r#"
            SELECT
              a.id,
              a.village_id,
              a.player_id,
              a.current_map_field_id,
              a.tribe as "tribe: _",
              a.units,
              a.smithy,
              a.hero_id as "hero_id?: Uuid",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.level          END as "hero_level?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.health         END as "hero_health?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.experience     END as "hero_experience?: i32",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.resource_focus END as "hero_resource_focus?: _",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.strength_points       END as "hero_strength_points?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.resources_points      END as "hero_resources_points?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.regeneration_points   END as "hero_regeneration_points?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.off_bonus_points      END as "hero_off_bonus_points?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.def_bonus_points      END as "hero_def_bonus_points?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.unassigned_points      END as "hero_unassigned_points?: i16"
            FROM armies a
            LEFT JOIN heroes h ON a.hero_id = h.id
            WHERE a.id = $1
            "#,
            army_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::ArmyNotFound(army_id)))?;

        Ok(army.into())
    }

    async fn get_by_hero_id(&self, hero_id: Uuid) -> Result<Army, ApplicationError> {
        let mut tx = self.tx.lock().await;
        let army = sqlx::query_as!(
            db_models::Army,
            r#"
            SELECT
              a.id,
              a.village_id,
              a.player_id,
              a.current_map_field_id,
              a.tribe as "tribe: _",
              a.units,
              a.smithy,
              a.hero_id as "hero_id?: Uuid",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.level          END as "hero_level?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.health         END as "hero_health?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.experience     END as "hero_experience?: i32",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.resource_focus END as "hero_resource_focus?: _",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.strength_points       END as "hero_strength_points?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.resources_points      END as "hero_resources_points?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.regeneration_points   END as "hero_regeneration_points?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.off_bonus_points      END as "hero_off_bonus_points?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.def_bonus_points      END as "hero_def_bonus_points?: i16",
              CASE WHEN h.id IS NULL THEN NULL ELSE h.unassigned_points      END as "hero_unassigned_points?: i16"
            FROM armies a
            LEFT JOIN heroes h ON a.hero_id = h.id
            WHERE a.hero_id = $1
            "#,
            hero_id
        )
        .fetch_optional(&mut *tx.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::HeroNotFound(hero_id)))?;

        if let Some(army) = army {
            return Ok(army.into());
        } else {
            return Err(ApplicationError::Db(DbError::HeroWithoutArmy(hero_id)));
        }
    }

    async fn set_hero(&self, army_id: Uuid, hero_id: Option<Uuid>) -> Result<(), ApplicationError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"UPDATE armies SET hero_id = $2 WHERE id = $1"#,
            army_id,
            hero_id
        )
        .execute(&mut *tx.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn save(&self, army: &Army) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let db_tribe: Tribe = army.tribe.clone().into();
        let current_map_field_id: Option<i32> = army.current_map_field_id.map(|id| id as i32);
        let hero_id = army.hero().map(|h| h.id);

        sqlx::query!(
                r#"
                INSERT INTO armies (id, village_id, current_map_field_id, hero_id, units, smithy, tribe, player_id)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (id) DO UPDATE
                SET
                    units = $5,
                    hero_id = $4,
                    current_map_field_id = $3,
                    village_id = $2,
                    player_id = $8,
                    tribe = $7,
                    smithy = $6
                "#,
                army.id,
                army.village_id as i32,
                current_map_field_id,
                hero_id,
                Json(&army.units()) as _,
                Json(&army.smithy()) as _,
                db_tribe as _,
                army.player_id
            )
            .execute(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn remove(&self, army_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        sqlx::query!(r#"DELETE FROM armies WHERE id = $1"#, army_id)
            .execute(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }
}
