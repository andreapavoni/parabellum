//! Write helpers for village movement projections.

use parabellum_app::villages::models::VillageMovement;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::{PostgresVillageMovementRepository, queries};

impl PostgresVillageMovementRepository {
    /// Upserts one village movement row inside an existing transaction.
    pub async fn upsert_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        movement: &VillageMovement,
    ) -> Result<(), ApplicationError> {
        queries::upsert_village_movement_query(movement)
            .build()
            .execute(&mut **tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    /// Deletes all direction rows for one movement inside an existing transaction.
    pub async fn delete_by_movement_id_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        movement_id: Uuid,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            DELETE FROM rm_village_movements
            WHERE movement_id = $1
            "#,
        )
        .bind(movement_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }
}
