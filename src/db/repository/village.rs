use sqlx::{Postgres, Transaction, types::Json};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    Result,
    db::{DbError, mapping::VillageAggregate, models as db_models},
    error::ApplicationError,
    game::models::village::Village,
    repository::VillageRepository,
};

/// Implements VillageRepository and operates on transactions.
#[derive(Clone)]
pub struct PostgresVillageRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresVillageRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> VillageRepository for PostgresVillageRepository<'a> {
    async fn get_by_id(&self, village_id_u32: u32) -> Result<Village, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let village_id_i32 = village_id_u32 as i32;

        let db_village = sqlx::query_as!(
            db_models::Village,
            "SELECT * FROM villages WHERE id = $1",
            village_id_i32
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(village_id_u32)))?;

        let db_player = sqlx::query_as!(
            db_models::Player,
            r#"SELECT id, username, tribe AS "tribe: _" FROM players WHERE id = $1"#,
            db_village.player_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let all_armies = sqlx::query_as!(
                  db_models::Army,
                  r#"SELECT id, village_id, current_map_field_id, hero_id, units, smithy, player_id, tribe AS "tribe: _" FROM armies WHERE village_id = $1 OR current_map_field_id = $1"#,
                  village_id_i32
              )
              .fetch_all(&mut *tx_guard.as_mut())
              .await.map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let db_oases = sqlx::query_as!(
            db_models::MapField,
            "SELECT * FROM map_fields WHERE village_id = $1",
            village_id_i32
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let aggregate = VillageAggregate {
            village: db_village,
            player: db_player,
            armies: all_armies,
            oases: db_oases,
        };

        let mut game_village = Village::try_from(aggregate)?;
        game_village.update_state();
        Ok(game_village)
    }

    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Village>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let villages_ids = sqlx::query!("SELECT id FROM villages WHERE player_id = $1", player_id)
            .fetch_all(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let mut result = Vec::new();
        for record in villages_ids {
            let village = self.get_by_id(record.id as u32).await?;
            result.push(village);
        }

        Ok(result)
    }

    async fn save(&self, village: &Village) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query!(
            r#"
                INSERT INTO villages (
                    id, player_id, name, position, buildings, production,
                    stocks, smithy_upgrades, academy_research, population,
                    loyalty, is_capital
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (id) DO UPDATE
                SET
                    name = $3,
                    buildings = $5,
                    production = $6,
                    stocks = $7,
                    smithy_upgrades = $8,
                    academy_research = $9,
                    population = $10,
                    loyalty = $11,
                    updated_at = NOW()
                "#,
            village.id as i32,
            village.player_id,
            village.name,
            Json(&village.position) as _,
            Json(&village.buildings) as _,
            Json(&village.production) as _,
            Json(&village.stocks) as _,
            Json(&village.smithy) as _,
            Json(&village.academy_research) as _,
            village.population as i32,
            village.loyalty as i16,
            village.is_capital
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }
}
