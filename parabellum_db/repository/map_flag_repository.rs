use parabellum_game::models::map_flag::MapFlag;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::MapFlagRepository;
use parabellum_types::errors::{ApplicationError, DbError, GameError, Result};

use crate::models::{self as db_models};

/// Implements MapFlagRepository and operates on transactions.
#[derive(Clone)]
pub struct PostgresMapFlagRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresMapFlagRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> MapFlagRepository for PostgresMapFlagRepository<'a> {
    async fn save(&self, flag: &MapFlag) -> Result<(), ApplicationError> {
        let flag_type: db_models::MapFlagType = flag.flag_type.into();
        let mut tx_guard = self.tx.lock().await;

        let position_json = flag.position.as_ref().map(|p| serde_json::to_value(p).expect("Failed to serialize position"));

        sqlx::query!(
            r#"
            INSERT INTO map_flag (
                id, alliance_id, player_id, target_id, position,
                flag_type, color, text, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE
            SET
                color = $7,
                text = $8
            "#,
            flag.id,
            flag.alliance_id,
            flag.player_id,
            flag.target_id,
            position_json,
            flag_type as _,
            flag.color,
            flag.text,
            flag.created_by,
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<MapFlag, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let flag = sqlx::query_as::<_, db_models::MapFlag>(
            r#"
            SELECT id, alliance_id, player_id, target_id, position,
                   flag_type, color, text, created_by, created_at, updated_at
            FROM map_flag
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::MapFlagNotFound(id)))?;

        Ok(flag.into())
    }

    async fn get_by_player_id(&self, player_id: Uuid) -> Result<Vec<MapFlag>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let flags = sqlx::query_as::<_, db_models::MapFlag>(
            r#"
            SELECT id, alliance_id, player_id, target_id, position,
                   flag_type, color, text, created_by, created_at, updated_at
            FROM map_flag
            WHERE player_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(player_id)
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(flags.into_iter().map(|f| f.into()).collect())
    }

    async fn get_by_alliance_id(&self, alliance_id: Uuid) -> Result<Vec<MapFlag>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let flags = sqlx::query_as::<_, db_models::MapFlag>(
            r#"
            SELECT id, alliance_id, player_id, target_id, position,
                   flag_type, color, text, created_by, created_at, updated_at
            FROM map_flag
            WHERE alliance_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(alliance_id)
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(flags.into_iter().map(|f| f.into()).collect())
    }

    async fn get_by_coordinates(&self, x: i32, y: i32) -> Result<Vec<MapFlag>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let position_json = serde_json::json!({"x": x, "y": y});

        let flags = sqlx::query_as::<_, db_models::MapFlag>(
            r#"
            SELECT id, alliance_id, player_id, target_id, position,
                   flag_type, color, text, created_by, created_at, updated_at
            FROM map_flag
            WHERE position @> $1
            ORDER BY created_at DESC
            "#
        )
        .bind(position_json)
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(flags.into_iter().map(|f| f.into()).collect())
    }

    async fn get_by_target_id(&self, target_id: Uuid) -> Result<Vec<MapFlag>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let flags = sqlx::query_as::<_, db_models::MapFlag>(
            r#"
            SELECT id, alliance_id, player_id, target_id, position,
                   flag_type, color, text, created_by, created_at, updated_at
            FROM map_flag
            WHERE target_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(target_id)
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(flags.into_iter().map(|f| f.into()).collect())
    }

    async fn count_by_owner(
        &self,
        player_id: Option<Uuid>,
        alliance_id: Option<Uuid>,
        flag_type: Option<parabellum_types::map_flag::MapFlagType>,
    ) -> Result<i64, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let count: i64 = match (player_id, alliance_id, flag_type) {
            (Some(pid), None, Some(ftype)) => {
                let ftype: db_models::MapFlagType = ftype.into();
                sqlx::query_scalar!(
                    r#"
                    SELECT COUNT(*) as "count!"
                    FROM map_flag
                    WHERE player_id = $1 AND flag_type = $2
                    "#,
                    pid,
                    ftype as _
                )
                .fetch_one(&mut *tx_guard.as_mut())
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
            }
            (Some(pid), None, None) => {
                sqlx::query_scalar!(
                    r#"
                    SELECT COUNT(*) as "count!"
                    FROM map_flag
                    WHERE player_id = $1
                    "#,
                    pid
                )
                .fetch_one(&mut *tx_guard.as_mut())
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
            }
            (None, Some(aid), Some(ftype)) => {
                let ftype: db_models::MapFlagType = ftype.into();
                sqlx::query_scalar!(
                    r#"
                    SELECT COUNT(*) as "count!"
                    FROM map_flag
                    WHERE alliance_id = $1 AND flag_type = $2
                    "#,
                    aid,
                    ftype as _
                )
                .fetch_one(&mut *tx_guard.as_mut())
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
            }
            (None, Some(aid), None) => {
                sqlx::query_scalar!(
                    r#"
                    SELECT COUNT(*) as "count!"
                    FROM map_flag
                    WHERE alliance_id = $1
                    "#,
                    aid
                )
                .fetch_one(&mut *tx_guard.as_mut())
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
            }
            _ => {
                return Err(ApplicationError::Game(GameError::MapFlagInvalidOwnership))
            }
        };

        Ok(count)
    }

    async fn update(&self, flag: &MapFlag) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query!(
            r#"
            UPDATE map_flag
            SET color = $1, text = $2
            WHERE id = $3
            "#,
            flag.color,
            flag.text,
            flag.id
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query!(
            r#"
            DELETE FROM map_flag
            WHERE id = $1
            "#,
            id
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }
}
