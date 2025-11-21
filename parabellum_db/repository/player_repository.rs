use parabellum_game::models::player::Player;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::PlayerRepository;
use parabellum_core::{ApplicationError, DbError, Result};

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
              INSERT INTO players (id, username, tribe, user_id, alliance_id, alliance_join_time,
                  current_alliance_training_contributions, current_alliance_armor_contributions,
                  current_alliance_cp_contributions, current_alliance_trade_contributions,
                  total_alliance_training_contributions, total_alliance_armor_contributions,
                  total_alliance_cp_contributions, total_alliance_trade_contributions)
              VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
              ON CONFLICT (id) DO UPDATE
              SET
                  username = $2,
                  tribe = $3,
                  alliance_id = $5,
                  alliance_join_time = $6,
                  current_alliance_training_contributions = $7,
                  current_alliance_armor_contributions = $8,
                  current_alliance_cp_contributions = $9,
                  current_alliance_trade_contributions = $10,
                  total_alliance_training_contributions = $11,
                  total_alliance_armor_contributions = $12,
                  total_alliance_cp_contributions = $13,
                  total_alliance_trade_contributions = $14
              "#,
            player.id,
            player.username,
            tribe as _,
            player.user_id,
            player.alliance_id,
            player.alliance_join_time,
            player.current_alliance_training_contributions,
            player.current_alliance_armor_contributions,
            player.current_alliance_cp_contributions,
            player.current_alliance_trade_contributions,
            player.total_alliance_training_contributions,
            player.total_alliance_armor_contributions,
            player.total_alliance_cp_contributions,
            player.total_alliance_trade_contributions,
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let player = sqlx::query_as::<_, db_models::Player>(
            r#"SELECT id, username, tribe::text as tribe, user_id, created_at, alliance_id, alliance_role_name, alliance_role, alliance_join_time, alliance_contributions, current_alliance_training_contributions, current_alliance_armor_contributions, current_alliance_cp_contributions, current_alliance_trade_contributions, total_alliance_training_contributions, total_alliance_armor_contributions, total_alliance_cp_contributions, total_alliance_trade_contributions, alliance_notification_enabled, alliance_settings FROM players WHERE id = $1"#
        )
        .bind(player_id)
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::PlayerNotFound(player_id)))?;

        Ok(player.into())
    }

    async fn get_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let player = sqlx::query_as::<_, db_models::Player>(
            r#"SELECT id, username, tribe::text as tribe, user_id, created_at, alliance_id, alliance_role_name, alliance_role, alliance_join_time, alliance_contributions, current_alliance_training_contributions, current_alliance_armor_contributions, current_alliance_cp_contributions, current_alliance_trade_contributions, total_alliance_training_contributions, total_alliance_armor_contributions, total_alliance_cp_contributions, total_alliance_trade_contributions, alliance_notification_enabled, alliance_settings FROM players WHERE user_id = $1"#
        )
        .bind(user_id)
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::PlayerNotFound(user_id)))?;

        Ok(player.into())
    }
}
