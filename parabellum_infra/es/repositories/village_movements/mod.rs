//! Postgres implementation of projected village movement repositories.

mod queries;
mod rows;
mod writes;

use parabellum_app::villages::models::VillageMovement;
use parabellum_app::villages::projection_repositories::{
    VillageMovementFilter, VillageMovementRepository,
};
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ProjectionDb;

use rows::DbVillageMovementPayloadRow;

/// Postgres-backed repository for village movement projections.
#[derive(Debug, Clone)]
pub struct PostgresVillageMovementRepository {
    pool: PgPool,
}

impl PostgresVillageMovementRepository {
    /// Creates a village movement repository backed by the projection database.
    pub fn new(db: ProjectionDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }
}

#[async_trait::async_trait]
impl VillageMovementRepository for PostgresVillageMovementRepository {
    async fn upsert(&self, movement: &VillageMovement) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        self.upsert_in_tx(&mut tx, movement).await?;
        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn list_movements(
        &self,
        filter: VillageMovementFilter,
    ) -> Result<Vec<VillageMovement>, ApplicationError> {
        let rows: Vec<DbVillageMovementPayloadRow> = queries::village_movement_list_query(filter)
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn delete_by_movement_id(&self, movement_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        self.delete_by_movement_id_in_tx(&mut tx, movement_id)
            .await?;
        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }
}
