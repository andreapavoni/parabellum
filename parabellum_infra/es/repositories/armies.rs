use parabellum_app::villages::{
    VillageArmyContext,
    projection_repositories::{ArmyListFilter, ArmyRepository, ArmyState},
};
use parabellum_game::models::{
    army::Army,
    hero::{Hero, HeroResourceFocus},
    smithy::SmithyUpgrades,
};
use parabellum_types::{
    army::TroopSet,
    errors::{ApplicationError, DbError},
};
use sqlx::{PgPool, Postgres, QueryBuilder, Row, Transaction, postgres::PgRow};
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

    pub async fn upsert_trapped_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        army: &Army,
        trapped_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert_in_tx_inner(Some(tx), army, trapped_village_id, player_id, "trapped")
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
        let row = Self::army_query(
            ArmyListFilter::new()
                .army_id(army_id)
                .state(ArmyState::Stationed)
                .limit(1),
        )?
        .build()
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        row.map(|row| {
            let stationed_village_id = row.get::<i32, _>("current_village_id") as u32;
            Self::army_from_row(row).map(|army| (stationed_village_id, army))
        })
        .transpose()
    }

    async fn find_trapped_context(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, Army)>, ApplicationError> {
        let row = Self::army_query(
            ArmyListFilter::new()
                .army_id(army_id)
                .state(ArmyState::Trapped)
                .limit(1),
        )?
        .build()
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        row.map(|row| {
            let trapped_village_id = row.get::<i32, _>("current_village_id") as u32;
            Self::army_from_row(row).map(|army| (trapped_village_id, army))
        })
        .transpose()
    }

    fn army_query(
        filter: ArmyListFilter,
    ) -> Result<QueryBuilder<'static, Postgres>, ApplicationError> {
        let mut query = QueryBuilder::<Postgres>::new(army_select_sql());
        let mut has_where = false;

        if let Some(army_id) = filter.army_id {
            Self::push_filter(&mut query, &mut has_where);
            query.push("a.army_id = ");
            query.push_bind(army_id);
        }

        if let Some(home_village_id) = filter.home_village_id {
            Self::push_filter(&mut query, &mut has_where);
            query.push("a.village_id = ");
            query.push_bind(home_village_id as i32);
        }

        if let Some(current_village_id) = filter.current_village_id {
            Self::push_filter(&mut query, &mut has_where);
            query.push("a.current_village_id = ");
            query.push_bind(current_village_id as i32);
        }

        if let Some(state) = filter.state {
            Self::push_filter(&mut query, &mut has_where);
            query.push("a.state = ");
            query.push_bind(Self::state_name(state));
        }

        if let Some(deployed) = filter.deployed {
            let Some(home_village_id) = filter.home_village_id else {
                return Err(ApplicationError::Db(DbError::Database(
                    sqlx::Error::Protocol("army deployed filter requires home_village_id".into()),
                )));
            };
            Self::push_filter(&mut query, &mut has_where);
            if deployed {
                query.push("a.current_village_id <> ");
            } else {
                query.push("a.current_village_id = ");
            }
            query.push_bind(home_village_id as i32);
        }

        query.push(" ORDER BY a.updated_at DESC");

        if let Some(limit) = filter.limit {
            query.push(" LIMIT ");
            query.push_bind(limit);
        }

        Ok(query)
    }

    fn push_filter(query: &mut QueryBuilder<'static, Postgres>, has_where: &mut bool) {
        if *has_where {
            query.push(" AND ");
        } else {
            query.push(" WHERE ");
            *has_where = true;
        }
    }

    fn state_name(state: ArmyState) -> &'static str {
        match state {
            ArmyState::Home => "home",
            ArmyState::Stationed => "stationed",
            ArmyState::Moving => "moving",
            ArmyState::Trapped => "trapped",
        }
    }

    fn list_armies_from_rows(rows: Vec<PgRow>) -> Result<Vec<Army>, ApplicationError> {
        rows.into_iter().map(Self::army_from_row).collect()
    }

    pub async fn list_armies_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        filter: ArmyListFilter,
    ) -> Result<Vec<Army>, ApplicationError> {
        let rows = Self::army_query(filter)?
            .build()
            .fetch_all(&mut **tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Self::list_armies_from_rows(rows)
    }

    async fn get_moving_by_army_id(&self, army_id: Uuid) -> Result<Army, ApplicationError> {
        let mut armies = self
            .list_armies(
                ArmyListFilter::new()
                    .army_id(army_id)
                    .state(ArmyState::Moving)
                    .limit(1),
            )
            .await?;
        armies
            .pop()
            .ok_or(ApplicationError::Db(DbError::ArmyNotFound(army_id)))
    }

    pub async fn army_context_for_village_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
    ) -> Result<VillageArmyContext, ApplicationError> {
        let mut home_armies = self
            .list_armies_in_tx(
                tx,
                ArmyListFilter::new()
                    .home_village(village_id)
                    .current_village(village_id)
                    .state(ArmyState::Home)
                    .limit(1),
            )
            .await?;
        Ok(VillageArmyContext {
            home: home_armies.pop(),
            stationed: self
                .list_armies_in_tx(
                    tx,
                    ArmyListFilter::new()
                        .current_village(village_id)
                        .state(ArmyState::Stationed),
                )
                .await?,
            deployed: self
                .list_armies_in_tx(
                    tx,
                    ArmyListFilter::new()
                        .home_village(village_id)
                        .state(ArmyState::Stationed)
                        .deployed(true),
                )
                .await?,
            moving: self
                .list_armies_in_tx(
                    tx,
                    ArmyListFilter::new()
                        .home_village(village_id)
                        .state(ArmyState::Moving),
                )
                .await?,
            trapped_here: self
                .list_armies_in_tx(
                    tx,
                    ArmyListFilter::new()
                        .current_village(village_id)
                        .state(ArmyState::Trapped),
                )
                .await?,
            trapped_away: self
                .list_armies_in_tx(
                    tx,
                    ArmyListFilter::new()
                        .home_village(village_id)
                        .state(ArmyState::Trapped)
                        .deployed(true),
                )
                .await?,
        })
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

    async fn list_armies(&self, filter: ArmyListFilter) -> Result<Vec<Army>, ApplicationError> {
        let rows = Self::army_query(filter)?
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Self::list_armies_from_rows(rows)
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

    async fn find_trapped_context_by_army_id(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, Army)>, ApplicationError> {
        self.find_trapped_context(army_id).await
    }

    async fn army_context_for_village(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyContext, ApplicationError> {
        let mut home_armies = self
            .list_armies(
                ArmyListFilter::new()
                    .home_village(village_id)
                    .current_village(village_id)
                    .state(ArmyState::Home)
                    .limit(1),
            )
            .await?;
        Ok(VillageArmyContext {
            home: home_armies.pop(),
            stationed: self
                .list_armies(
                    ArmyListFilter::new()
                        .current_village(village_id)
                        .state(ArmyState::Stationed),
                )
                .await?,
            deployed: self
                .list_armies(
                    ArmyListFilter::new()
                        .home_village(village_id)
                        .state(ArmyState::Stationed)
                        .deployed(true),
                )
                .await?,
            moving: self
                .list_armies(
                    ArmyListFilter::new()
                        .home_village(village_id)
                        .state(ArmyState::Moving),
                )
                .await?,
            trapped_here: self
                .list_armies(
                    ArmyListFilter::new()
                        .current_village(village_id)
                        .state(ArmyState::Trapped),
                )
                .await?,
            trapped_away: self
                .list_armies(
                    ArmyListFilter::new()
                        .home_village(village_id)
                        .state(ArmyState::Trapped)
                        .deployed(true),
                )
                .await?,
        })
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
