//! Write helpers for village movement projections.

use parabellum_app::villages::models::{MovementDirection, VillageMovement};
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{Postgres, Transaction, types::Json};
use uuid::Uuid;

use super::{
    PostgresVillageMovementRepository,
    rows::{DbMovementDirection, DbMovementType},
};

impl PostgresVillageMovementRepository {
    /// Upserts one village movement row inside an existing transaction.
    pub async fn upsert_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        movement: &VillageMovement,
    ) -> Result<(), ApplicationError> {
        let village_id = match movement.direction {
            MovementDirection::Incoming => movement.target_village_id,
            MovementDirection::Outgoing => movement.origin_village_id,
        };

        sqlx::query(
            r#"
            INSERT INTO rm_village_movements (
                village_id, movement_id, direction, movement_type, source_village_id,
                target_village_id, eta, payload
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (village_id, movement_id, direction)
            DO UPDATE SET
                movement_type = EXCLUDED.movement_type,
                source_village_id = EXCLUDED.source_village_id,
                target_village_id = EXCLUDED.target_village_id,
                eta = EXCLUDED.eta,
                payload = EXCLUDED.payload,
                updated_at = NOW()
            "#,
        )
        .bind(village_id as i32)
        .bind(movement.movement_id)
        .bind(DbMovementDirection::from(movement.direction))
        .bind(DbMovementType::from(movement.movement_type))
        .bind(movement.origin_village_id as i32)
        .bind(movement.target_village_id as i32)
        .bind(movement.arrives_at)
        .bind(Json(movement))
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
