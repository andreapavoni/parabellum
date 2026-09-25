//! Village projection state-change helpers.

use parabellum_app::villages::models::VillageModel;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{Postgres, Transaction};

use super::{PostgresVillageRepository, writes};

impl PostgresVillageRepository {
    /// Stores a full village read-model row inside an existing transaction.
    pub async fn store_village_model_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        model: &VillageModel,
    ) -> Result<(), ApplicationError> {
        writes::store_village_model_query(model)
            .build()
            .execute(&mut **tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    /// Inserts or replaces a full village read-model row inside an existing transaction.
    pub async fn upsert_village_model_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        model: &VillageModel,
    ) -> Result<(), ApplicationError> {
        writes::upsert_village_model_query(model)
            .build()
            .execute(&mut **tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }
}
