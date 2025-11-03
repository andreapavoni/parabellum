use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    Result,
    db::{DbError, models as db_models},
    error::ApplicationError,
    game::models::Player,
    repository::PlayerRepository,
};

// Implements PlayerRepository and operates on transactions.
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
    async fn create(&self, player: &Player) -> Result<(), ApplicationError> {
        let tribe: db_models::Tribe = player.tribe.clone().into();

        // Get the lock on the transaction
        let mut tx_guard = self.tx.lock().await;

        sqlx::query_as!(
            db_models::Player,
            r#"
                INSERT INTO players (id, username, tribe)
                VALUES ($1, $2, $3)
                RETURNING id, username, tribe AS "tribe: _"
                "#,
            player.id,
            player.username,
            tribe as _
        )
        .fetch_one(&mut *tx_guard.as_mut()) // Use &mut *tx_guard as Executor
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(()) // release the lock when tx_guard goes out of scope
    }

    async fn get_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let player = sqlx::query_as!(
            db_models::Player,
            r#"SELECT id, username, tribe AS "tribe: _" FROM players WHERE id = $1"#,
            player_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::PlayerNotFound(player_id)))?;

        Ok(player.into())
    }
}
