use parabellum_types::common::Player;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::{PlayerLeaderboardEntry, PlayerRepository};
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
};

use crate::models::{self as db_models};

/// Implements PlayerRepository and operates on transactions.
#[derive(Clone)]
pub struct PostgresPlayerRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresPlayerRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> PlayerRepository for PostgresPlayerRepository<'a> {
    async fn save(&self, player: &Player) -> Result<(), ApplicationError> {
        let tribe: db_models::Tribe = player.tribe.clone().into();
        let mut tx_guard = self.tx.lock().await;

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
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let player = sqlx::query_as!(
            db_models::Player,
            r#"SELECT id, username, tribe AS "tribe: _", user_id, culture_points FROM players WHERE id = $1"#,
            player_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::PlayerNotFound(player_id)))?;

        Ok(player.into())
    }

    async fn get_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let player = sqlx::query_as!(
            db_models::Player,
            r#"SELECT id, username, tribe AS "tribe: _", user_id, culture_points FROM players WHERE user_id = $1"#,
            user_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::UserPlayerNotFound(user_id)))?;

        Ok(player.into())
    }

    async fn leaderboard_page(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<PlayerLeaderboardEntry>, i64), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        // Total player count for pagination
        let total_players = sqlx::query!("SELECT COUNT(*) as count FROM players")
            .fetch_one(&mut *tx_guard.as_mut())
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
                COUNT(v.id) as "village_count!: i64",
                COALESCE(SUM(v.population), 0) as "population!: i64"
            FROM players p
            LEFT JOIN villages v ON v.player_id = p.id
            GROUP BY p.id, p.username
            ORDER BY COALESCE(SUM(v.population), 0) DESC, p.username ASC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let entries = rows
            .into_iter()
            .map(|row| PlayerLeaderboardEntry {
                player_id: row.player_id,
                username: row.username,
                village_count: row.village_count,
                population: row.population,
                tribe: row.tribe.into(),
            })
            .collect();

        Ok((entries, total_players))
    }

    async fn update_culture_points(&self, player_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        // Sum culture_points from all villages owned by this player
        let total_cp = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(culture_points), 0) as "total!: i64"
            FROM villages
            WHERE player_id = $1
            "#,
            player_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
        .total;

        // Update player's culture_points
        sqlx::query!(
            r#"
            UPDATE players
            SET culture_points = $1
            WHERE id = $2
            "#,
            total_cp as i32,
            player_id
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_total_culture_points_production(
        &self,
        player_id: Uuid,
    ) -> Result<u32, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        // Sum culture_points_production from all villages owned by this player
        let total_cpp = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(culture_points_production), 0) as "total!: i64"
            FROM villages
            WHERE player_id = $1
            "#,
            player_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
        .total;

        Ok(total_cpp as u32)
    }
}
