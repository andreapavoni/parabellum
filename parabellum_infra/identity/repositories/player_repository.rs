use parabellum_types::common::Player;
use sqlx::PgPool;
use uuid::Uuid;

use parabellum_app::{
    identity::PlayerRepository, leaderboards::LeaderboardReadPort,
    read_models::PlayerPopulationLeaderboardEntry,
};
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
};
use sqlx::Row;

use crate::persistence::models::{self as db_models};

/// Implements PlayerRepository against identity + ES read-model tables.
#[derive(Debug, Clone)]
pub struct PostgresPlayerRepository {
    pool: PgPool,
}

impl PostgresPlayerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl PlayerRepository for PostgresPlayerRepository {
    async fn save(&self, player: &Player) -> Result<(), ApplicationError> {
        let tribe: db_models::Tribe = player.tribe.clone().into();

        sqlx::query!(
            r#"
              INSERT INTO players (id, username, tribe, user_id, culture_points)
              VALUES ($1, $2, $3, $4, $5)
              ON CONFLICT (id) DO UPDATE
              SET
                  username = $2,
                  tribe = $3,
                  culture_points = $5
              "#,
            player.id,
            player.username,
            tribe as _,
            player.user_id,
            player.culture_points as i32,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError> {
        let player = sqlx::query_as!(
            db_models::Player,
            r#"SELECT id, username, tribe AS "tribe: _", user_id, culture_points FROM players WHERE id = $1"#,
            player_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ApplicationError::Db(DbError::PlayerNotFound(player_id)))?;

        Ok(player.into())
    }

    async fn get_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError> {
        let player = sqlx::query_as!(
            db_models::Player,
            r#"SELECT id, username, tribe AS "tribe: _", user_id, culture_points FROM players WHERE user_id = $1"#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ApplicationError::Db(DbError::UserPlayerNotFound(user_id)))?;

        Ok(player.into())
    }

    async fn update_culture_points(&self, player_id: Uuid) -> Result<(), ApplicationError> {
        // Sum CPP/day across all villages owned by this player.
        let total_cpp = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(culture_points_production), 0) as "total!: i64"
            FROM rm_village
            WHERE player_id = $1
            "#,
            player_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
        .total;

        let player_row = sqlx::query(
            r#"
            SELECT culture_points, culture_points_updated_at
            FROM players
            WHERE id = $1
            "#,
        )
        .bind(player_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let current_cp: i32 = player_row.get("culture_points");
        let cp_updated_at: chrono::DateTime<chrono::Utc> =
            player_row.get("culture_points_updated_at");
        let now = chrono::Utc::now();
        let elapsed_secs = (now - cp_updated_at).num_seconds();

        if elapsed_secs <= 0 || total_cpp <= 0 {
            return Ok(());
        }

        let cp_delta = ((total_cpp as f64 / 86_400.0) * elapsed_secs as f64).floor() as i32;
        if cp_delta <= 0 {
            return Ok(());
        }

        sqlx::query(
            r#"
            UPDATE players
            SET culture_points = $1,
                culture_points_updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(current_cp.saturating_add(cp_delta))
        .bind(player_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_total_culture_points_production(
        &self,
        player_id: Uuid,
    ) -> Result<u32, ApplicationError> {
        // Sum culture_points_production from all ES village read models owned by this player.
        let total_cpp = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(culture_points_production), 0) as "total!: i64"
            FROM rm_village
            WHERE player_id = $1
            "#,
            player_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
        .total;

        Ok(total_cpp as u32)
    }
}

#[async_trait::async_trait]
impl LeaderboardReadPort for PostgresPlayerRepository {
    async fn list_player_population_entries(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<PlayerPopulationLeaderboardEntry>, i64), ApplicationError> {
        let total_players = sqlx::query!("SELECT COUNT(*) as count FROM players")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
            .count
            .unwrap_or(0);

        #[derive(Debug)]
        struct LeaderboardRow {
            player_id: Uuid,
            username: String,
            tribe: db_models::Tribe,
            village_count: i64,
            population: i64,
        }

        let rows = sqlx::query_as!(
            LeaderboardRow,
            r#"
            SELECT
                p.id as player_id,
                p.username,
                p.tribe as "tribe: _",
                COUNT(v.village_id) as "village_count!: i64",
                COALESCE(SUM(v.population), 0) as "population!: i64"
            FROM players p
            LEFT JOIN rm_village v ON v.player_id = p.id
            GROUP BY p.id, p.username
            ORDER BY COALESCE(SUM(v.population), 0) DESC, p.username ASC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let entries = rows
            .into_iter()
            .map(|row| PlayerPopulationLeaderboardEntry {
                player_id: row.player_id,
                username: row.username,
                village_count: row.village_count,
                population: row.population,
                tribe: row.tribe.into(),
            })
            .collect();

        Ok((entries, total_players))
    }
}
