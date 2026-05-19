use parabellum_app::villages::repositories::HeroRepository;
use parabellum_game::models::hero::{Hero, HeroResourceFocus};
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresHeroRepository {
    pool: PgPool,
}

impl PostgresHeroRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        hero: &Hero,
        home_village_id: u32,
        current_village_id: u32,
        state: &str,
    ) -> Result<(), ApplicationError> {
        let tribe: crate::persistence::models::Tribe = hero.tribe.clone().into();
        sqlx::query(
            r#"
            INSERT INTO rm_heroes (
                hero_id, player_id, home_village_id, current_village_id, state, tribe, level,
                health, experience, resource_focus, strength_points, off_bonus_points,
                def_bonus_points, regeneration_points, resources_points, unassigned_points
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10::jsonb,$11,$12,$13,$14,$15,$16
            )
            ON CONFLICT (hero_id) DO UPDATE SET
                player_id = EXCLUDED.player_id,
                home_village_id = EXCLUDED.home_village_id,
                current_village_id = EXCLUDED.current_village_id,
                state = EXCLUDED.state,
                tribe = EXCLUDED.tribe,
                level = EXCLUDED.level,
                health = EXCLUDED.health,
                experience = EXCLUDED.experience,
                resource_focus = EXCLUDED.resource_focus,
                strength_points = EXCLUDED.strength_points,
                off_bonus_points = EXCLUDED.off_bonus_points,
                def_bonus_points = EXCLUDED.def_bonus_points,
                regeneration_points = EXCLUDED.regeneration_points,
                resources_points = EXCLUDED.resources_points,
                unassigned_points = EXCLUDED.unassigned_points,
                updated_at = NOW()
            "#,
        )
        .bind(hero.id)
        .bind(hero.player_id)
        .bind(home_village_id as i32)
        .bind(current_village_id as i32)
        .bind(state)
        .bind(tribe as crate::persistence::models::Tribe)
        .bind(hero.level as i16)
        .bind(hero.health as i16)
        .bind(hero.experience as i32)
        .bind(serde_json::to_string(&hero.resource_focus).map_err(|e| {
            ApplicationError::Db(DbError::Database(sqlx::Error::Decode(Box::new(e))))
        })?)
        .bind(hero.strength_points as i16)
        .bind(hero.off_bonus_points as i16)
        .bind(hero.def_bonus_points as i16)
        .bind(hero.regeneration_points as i16)
        .bind(hero.resources_points as i16)
        .bind(hero.unassigned_points as i16)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl HeroRepository for PostgresHeroRepository {
    async fn upsert(
        &self,
        hero: &Hero,
        home_village_id: u32,
        current_village_id: u32,
        state: &str,
    ) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        self.upsert_in_tx(&mut tx, hero, home_village_id, current_village_id, state)
            .await?;
        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn get_by_id(&self, hero_id: Uuid) -> Result<Hero, ApplicationError> {
        let row = sqlx::query(
            r#"
            SELECT hero_id, player_id, home_village_id, tribe,
                   level, health, experience, resource_focus, strength_points, off_bonus_points,
                   def_bonus_points, regeneration_points, resources_points, unassigned_points
            FROM rm_heroes
            WHERE hero_id = $1
            "#,
        )
        .bind(hero_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ApplicationError::Db(DbError::HeroNotFound(hero_id)))?;

        let resource_focus: HeroResourceFocus = serde_json::from_value(row.get("resource_focus"))
            .map_err(|e| {
            ApplicationError::Db(DbError::Database(sqlx::Error::Decode(Box::new(e))))
        })?;
        let tribe: crate::persistence::models::Tribe = row.get("tribe");

        Ok(Hero {
            id: row.get("hero_id"),
            player_id: row.get("player_id"),
            village_id: row.get::<i32, _>("home_village_id") as u32,
            tribe: tribe.into(),
            level: row.get::<i16, _>("level") as u16,
            resource_focus,
            health: row.get::<i16, _>("health") as u16,
            experience: row.get::<i32, _>("experience") as u32,
            strength_points: row.get::<i16, _>("strength_points") as u16,
            off_bonus_points: row.get::<i16, _>("off_bonus_points") as u16,
            def_bonus_points: row.get::<i16, _>("def_bonus_points") as u16,
            regeneration_points: row.get::<i16, _>("regeneration_points") as u16,
            resources_points: row.get::<i16, _>("resources_points") as u16,
            unassigned_points: row.get::<i16, _>("unassigned_points") as u16,
        })
    }

    async fn get_by_player(&self, player_id: Uuid) -> Result<Option<Hero>, ApplicationError> {
        let row = sqlx::query(
            r#"
            SELECT hero_id, player_id, home_village_id, tribe,
                   level, health, experience, resource_focus, strength_points, off_bonus_points,
                   def_bonus_points, regeneration_points, resources_points, unassigned_points
            FROM rm_heroes
            WHERE player_id = $1
            "#,
        )
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let resource_focus: HeroResourceFocus = serde_json::from_value(row.get("resource_focus"))
            .map_err(|e| {
            ApplicationError::Db(DbError::Database(sqlx::Error::Decode(Box::new(e))))
        })?;
        let tribe: crate::persistence::models::Tribe = row.get("tribe");

        Ok(Some(Hero {
            id: row.get("hero_id"),
            player_id: row.get("player_id"),
            village_id: row.get::<i32, _>("home_village_id") as u32,
            tribe: tribe.into(),
            level: row.get::<i16, _>("level") as u16,
            resource_focus,
            health: row.get::<i16, _>("health") as u16,
            experience: row.get::<i32, _>("experience") as u32,
            strength_points: row.get::<i16, _>("strength_points") as u16,
            off_bonus_points: row.get::<i16, _>("off_bonus_points") as u16,
            def_bonus_points: row.get::<i16, _>("def_bonus_points") as u16,
            regeneration_points: row.get::<i16, _>("regeneration_points") as u16,
            resources_points: row.get::<i16, _>("resources_points") as u16,
            unassigned_points: row.get::<i16, _>("unassigned_points") as u16,
        }))
    }

    async fn has_alive_for_player(&self, player_id: Uuid) -> Result<bool, ApplicationError> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM rm_heroes
                WHERE player_id = $1
                  AND health > 0
            )
            "#,
        )
        .bind(player_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(exists)
    }
}
