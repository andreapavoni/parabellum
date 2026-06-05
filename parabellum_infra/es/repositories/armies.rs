use parabellum_app::villages::repositories::ArmyRepository;
use parabellum_game::models::{
    army::Army,
    hero::{Hero, HeroResourceFocus},
    smithy::SmithyUpgrades,
};
use parabellum_types::{
    army::TroopSet,
    errors::{ApplicationError, DbError},
};
use sqlx::{PgPool, Postgres, Row, Transaction, postgres::PgRow};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PostgresArmyRepository {
    pool: PgPool,
}

impl PostgresArmyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn upsert(
        &self,
        army: &Army,
        current_village_id: u32,
        player_id: Uuid,
        state: &str,
    ) -> Result<(), ApplicationError> {
        self.upsert_in_tx_inner(None, army, current_village_id, player_id, state)
            .await
    }

    async fn upsert_in_tx_inner(
        &self,
        tx: Option<&mut Transaction<'_, Postgres>>,
        army: &Army,
        current_village_id: u32,
        player_id: Uuid,
        state: &str,
    ) -> Result<(), ApplicationError> {
        let units: Vec<i32> = army.units().units().iter().map(|v| *v as i32).collect();
        let smithy_upgrades: Vec<i16> = army.smithy().iter().map(|v| *v as i16).collect();
        let hero_id = army.hero().map(|hero| hero.id);
        let tribe: crate::persistence::models::Tribe = army.tribe.clone().into();
        let q = sqlx::query(
            r#"
            INSERT INTO rm_armies (
                army_id, village_id, current_village_id, current_map_field_id, player_id, tribe,
                state, units, smithy_upgrades, hero_id, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
            ON CONFLICT (army_id) DO UPDATE SET
                village_id = EXCLUDED.village_id,
                current_village_id = EXCLUDED.current_village_id,
                current_map_field_id = EXCLUDED.current_map_field_id,
                player_id = EXCLUDED.player_id,
                tribe = EXCLUDED.tribe,
                state = EXCLUDED.state,
                units = EXCLUDED.units,
                smithy_upgrades = EXCLUDED.smithy_upgrades,
                hero_id = EXCLUDED.hero_id,
                updated_at = NOW()
            "#,
        )
        .bind(army.id)
        .bind(army.village_id as i32)
        .bind(current_village_id as i32)
        .bind(army.current_map_field_id.map(|id| id as i32))
        .bind(player_id)
        .bind(tribe)
        .bind(state)
        .bind(units)
        .bind(smithy_upgrades)
        .bind(hero_id);
        if let Some(tx) = tx {
            q.execute(&mut **tx)
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        } else {
            q.execute(&self.pool)
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        }
        Ok(())
    }

    async fn delete_row(&self, army_id: Uuid) -> Result<(), ApplicationError> {
        self.delete_row_in_tx_inner(None, army_id).await
    }

    async fn delete_row_in_tx_inner(
        &self,
        tx: Option<&mut Transaction<'_, Postgres>>,
        army_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let q = sqlx::query("DELETE FROM rm_armies WHERE army_id = $1").bind(army_id);
        if let Some(tx) = tx {
            q.execute(&mut **tx)
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        } else {
            q.execute(&self.pool)
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        }
        Ok(())
    }

    pub async fn upsert_home_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        army: &Army,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            DELETE FROM rm_armies
            WHERE village_id = $1
              AND state = 'home'
              AND army_id <> $2
            "#,
        )
        .bind(army.village_id as i32)
        .bind(army.id)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        self.upsert_in_tx_inner(Some(tx), army, army.village_id, player_id, "home")
            .await
    }

    pub async fn upsert_moving_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        army: &Army,
        current_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert_in_tx_inner(Some(tx), army, current_village_id, player_id, "moving")
            .await
    }

    pub async fn upsert_stationed_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        army: &Army,
        stationed_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert_in_tx_inner(Some(tx), army, stationed_village_id, player_id, "stationed")
            .await
    }

    pub async fn delete_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        army_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.delete_row_in_tx_inner(Some(tx), army_id).await
    }

    pub async fn delete_by_home_village_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
    ) -> Result<(), ApplicationError> {
        sqlx::query("DELETE FROM rm_armies WHERE village_id = $1")
            .bind(village_id as i32)
            .execute(&mut **tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn find_stationed_context(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, Army)>, ApplicationError> {
        let row = sqlx::query(&format!(
            "{} WHERE a.army_id = $1 AND a.state = 'stationed'",
            army_select_sql()
        ))
        .bind(army_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        row.map(|row| {
            let stationed_village_id = row.get::<i32, _>("current_village_id") as u32;
            Self::army_from_row(row).map(|army| (stationed_village_id, army))
        })
        .transpose()
    }

    pub async fn get_home_army_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
    ) -> Result<Option<Army>, ApplicationError> {
        let row = sqlx::query(&format!(
            "{} WHERE a.village_id = $1 AND a.current_village_id = $1 AND a.state = 'home'
            ORDER BY a.updated_at DESC LIMIT 1",
            army_select_sql()
        ))
        .bind(village_id as i32)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        row.map(Self::army_from_row).transpose()
    }

    pub async fn list_stationed_armies_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
    ) -> Result<Vec<Army>, ApplicationError> {
        let rows = sqlx::query(&format!(
            "{} WHERE a.current_village_id = $1 AND a.state = 'stationed'
            ORDER BY a.updated_at DESC",
            army_select_sql()
        ))
        .bind(village_id as i32)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        rows.into_iter().map(Self::army_from_row).collect()
    }

    pub async fn list_deployed_armies_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        home_village_id: u32,
    ) -> Result<Vec<Army>, ApplicationError> {
        let rows = sqlx::query(&format!(
            "{} WHERE a.village_id = $1 AND a.state = 'stationed' AND a.current_village_id <> $1
            ORDER BY a.updated_at DESC",
            army_select_sql()
        ))
        .bind(home_village_id as i32)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        rows.into_iter().map(Self::army_from_row).collect()
    }

    async fn get_moving_by_army_id(&self, army_id: Uuid) -> Result<Army, ApplicationError> {
        let row = sqlx::query(&format!(
            "{} WHERE a.army_id = $1 AND a.state = 'moving'",
            army_select_sql()
        ))
        .bind(army_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        row.map(Self::army_from_row)
            .transpose()?
            .ok_or(ApplicationError::Db(DbError::ArmyNotFound(army_id)))
    }

    fn troop_set(values: Vec<i32>) -> TroopSet {
        let mut units = [0_u32; 10];
        for (idx, value) in values.into_iter().take(10).enumerate() {
            units[idx] = value.max(0) as u32;
        }
        TroopSet::new(units)
    }

    fn smithy_upgrades(values: Vec<i16>) -> SmithyUpgrades {
        let mut upgrades = [0_u8; 8];
        for (idx, value) in values.into_iter().take(8).enumerate() {
            upgrades[idx] = value.max(0) as u8;
        }
        upgrades
    }

    fn hero_from_row(row: &PgRow) -> Result<Option<Hero>, ApplicationError> {
        let hero_id: Option<Uuid> = row.try_get("hero_id").map_err(database_decode)?;
        let Some(hero_id) = hero_id else {
            return Ok(None);
        };
        let resource_focus: serde_json::Value = row
            .try_get("hero_resource_focus")
            .map_err(database_decode)?;
        let resource_focus: HeroResourceFocus =
            serde_json::from_value(resource_focus).map_err(|e| {
                ApplicationError::Db(DbError::Database(sqlx::Error::Decode(Box::new(e))))
            })?;
        let tribe: crate::persistence::models::Tribe =
            row.try_get("hero_tribe").map_err(database_decode)?;
        Ok(Some(Hero {
            id: hero_id,
            player_id: row.try_get("hero_player_id").map_err(database_decode)?,
            village_id: row
                .try_get::<i32, _>("hero_home_village_id")
                .map_err(database_decode)? as u32,
            tribe: tribe.into(),
            level: row
                .try_get::<i16, _>("hero_level")
                .map_err(database_decode)? as u16,
            resource_focus,
            health: row
                .try_get::<i16, _>("hero_health")
                .map_err(database_decode)? as u16,
            experience: row
                .try_get::<i32, _>("hero_experience")
                .map_err(database_decode)? as u32,
            strength_points: row
                .try_get::<i16, _>("hero_strength_points")
                .map_err(database_decode)? as u16,
            off_bonus_points: row
                .try_get::<i16, _>("hero_off_bonus_points")
                .map_err(database_decode)? as u16,
            def_bonus_points: row
                .try_get::<i16, _>("hero_def_bonus_points")
                .map_err(database_decode)? as u16,
            regeneration_points: row
                .try_get::<i16, _>("hero_regeneration_points")
                .map_err(database_decode)? as u16,
            resources_points: row
                .try_get::<i16, _>("hero_resources_points")
                .map_err(database_decode)? as u16,
            unassigned_points: row
                .try_get::<i16, _>("hero_unassigned_points")
                .map_err(database_decode)? as u16,
        }))
    }

    fn army_from_row(row: PgRow) -> Result<Army, ApplicationError> {
        let tribe: crate::persistence::models::Tribe =
            row.try_get("tribe").map_err(database_decode)?;
        let units = Self::troop_set(row.try_get("units").map_err(database_decode)?);
        let smithy =
            Self::smithy_upgrades(row.try_get("smithy_upgrades").map_err(database_decode)?);
        let hero = Self::hero_from_row(&row)?;
        Ok(Army::new(
            Some(row.try_get("army_id").map_err(database_decode)?),
            row.try_get::<i32, _>("village_id")
                .map_err(database_decode)? as u32,
            row.try_get::<Option<i32>, _>("current_map_field_id")
                .map_err(database_decode)?
                .map(|id| id as u32),
            row.try_get("player_id").map_err(database_decode)?,
            tribe.into(),
            &units,
            &smithy,
            hero,
        ))
    }
}

fn database_decode(error: sqlx::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Database(error))
}

fn army_select_sql() -> &'static str {
    r#"
    SELECT
      a.army_id,
      a.village_id,
      a.current_village_id,
      a.current_map_field_id,
      a.player_id,
      a.tribe,
      a.units,
      a.smithy_upgrades,
      a.hero_id,
      h.player_id AS hero_player_id,
      h.home_village_id AS hero_home_village_id,
      h.tribe AS hero_tribe,
      h.level AS hero_level,
      h.health AS hero_health,
      h.experience AS hero_experience,
      h.resource_focus AS hero_resource_focus,
      h.strength_points AS hero_strength_points,
      h.off_bonus_points AS hero_off_bonus_points,
      h.def_bonus_points AS hero_def_bonus_points,
      h.regeneration_points AS hero_regeneration_points,
      h.resources_points AS hero_resources_points,
      h.unassigned_points AS hero_unassigned_points
    FROM rm_armies a
    LEFT JOIN rm_heroes h ON h.hero_id = a.hero_id
    "#
}

#[async_trait::async_trait]
impl ArmyRepository for PostgresArmyRepository {
    async fn upsert_home(&self, army: &Army, player_id: Uuid) -> Result<(), ApplicationError> {
        // Keep exactly one canonical home army row per village.
        sqlx::query(
            r#"
            DELETE FROM rm_armies
            WHERE village_id = $1
              AND state = 'home'
              AND army_id <> $2
            "#,
        )
        .bind(army.village_id as i32)
        .bind(army.id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        self.upsert(army, army.village_id, player_id, "home").await
    }

    async fn upsert_moving(
        &self,
        army: &Army,
        current_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert(army, current_village_id, player_id, "moving")
            .await
    }

    async fn upsert_stationed(
        &self,
        army: &Army,
        stationed_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert(army, stationed_village_id, player_id, "stationed")
            .await
    }

    async fn delete(&self, army_id: Uuid) -> Result<(), ApplicationError> {
        self.delete_row(army_id).await
    }

    async fn get_home_army(&self, village_id: u32) -> Result<Option<Army>, ApplicationError> {
        let row = sqlx::query(&format!(
            "{} WHERE a.village_id = $1 AND a.current_village_id = $1 AND a.state = 'home'
            ORDER BY a.updated_at DESC LIMIT 1",
            army_select_sql()
        ))
        .bind(village_id as i32)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        row.map(Self::army_from_row).transpose()
    }

    async fn list_stationed_armies(&self, village_id: u32) -> Result<Vec<Army>, ApplicationError> {
        let rows = sqlx::query(&format!(
            "{} WHERE a.current_village_id = $1 AND a.state = 'stationed'
            ORDER BY a.updated_at DESC",
            army_select_sql()
        ))
        .bind(village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        rows.into_iter().map(Self::army_from_row).collect()
    }

    async fn list_deployed_armies(
        &self,
        home_village_id: u32,
    ) -> Result<Vec<Army>, ApplicationError> {
        let rows = sqlx::query(&format!(
            "{} WHERE a.village_id = $1 AND a.state = 'stationed' AND a.current_village_id <> $1
            ORDER BY a.updated_at DESC",
            army_select_sql()
        ))
        .bind(home_village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        rows.into_iter().map(Self::army_from_row).collect()
    }

    async fn get_moving_army(&self, army_id: Uuid) -> Result<Army, ApplicationError> {
        self.get_moving_by_army_id(army_id).await
    }

    async fn find_stationed_context_by_army_id(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, Army)>, ApplicationError> {
        self.find_stationed_context(army_id).await
    }

    async fn delete_by_home_village(&self, village_id: u32) -> Result<(), ApplicationError> {
        sqlx::query("DELETE FROM rm_armies WHERE village_id = $1")
            .bind(village_id as i32)
            .execute(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }
}
