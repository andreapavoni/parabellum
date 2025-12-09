use parabellum_types::common::Player;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::PlayerRepository;
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
              INSERT INTO players (id, username, tribe, user_id)
              VALUES ($1, $2, $3, $4)
              ON CONFLICT (id) DO UPDATE
              SET
                  username = $2,
                  tribe = $3
              "#,
            player.id,
            player.username,
            tribe as _,
            player.user_id,
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
            r#"SELECT id, username, tribe AS "tribe: _", user_id FROM players WHERE id = $1"#,
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
            r#"SELECT id, username, tribe AS "tribe: _", user_id FROM players WHERE user_id = $1"#,
            user_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::UserPlayerNotFound(user_id)))?;

        Ok(player.into())
    }
}
