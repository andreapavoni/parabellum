use sqlx::{Postgres, Transaction, types::Json};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    Result,
    db::{
        DbError,
        models::{self as db_models, Tribe},
    },
    error::ApplicationError,
    game::models::army::Army,
    repository::ArmyRepository,
};

#[derive(Clone)]
pub struct PostgresArmyRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresArmyRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> ArmyRepository for PostgresArmyRepository<'a> {
    async fn get_by_id(&self, army_id: Uuid) -> Result<Army, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let army = sqlx::query_as!(
          db_models::Army,
          r#"SELECT id, village_id, current_map_field_id, hero_id, units, smithy, player_id, tribe AS "tribe: _" FROM armies WHERE id = $1"#,
          army_id
      )
      .fetch_one(&mut *tx_guard.as_mut())
      .await.map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(army.into())
    }

    async fn save(&self, army: &Army) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let db_tribe: Tribe = army.tribe.clone().into();
        let current_map_field_id = army.current_map_field_id.unwrap_or(army.village_id);
        let hero_id = army.hero.as_ref().map(|h| h.id);

        // Questa è la query UPSERT
        sqlx::query!(
                r#"
                INSERT INTO armies (id, village_id, current_map_field_id, hero_id, units, smithy, tribe, player_id)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (id) DO UPDATE
                SET
                    units = $5,
                    hero_id = $4,
                    current_map_field_id = $3,
                    -- Aggiungi altri campi che un 'save' dovrebbe aggiornare se necessario
                    village_id = $2,
                    player_id = $8,
                    tribe = $7,
                    smithy = $6
                "#,
                army.id,
                army.village_id as i32,
                current_map_field_id as i32,
                hero_id,
                Json(&army.units) as _,
                Json(&army.smithy) as _,
                db_tribe as _,
                army.player_id
            )
            .execute(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn remove(&self, army_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        sqlx::query!(r#"DELETE FROM armies WHERE id = $1"#, army_id)
            .execute(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }
}
