//! Postgres merchant movement repository.
//!
//! Active merchant movement views are derived from scheduled merchant actions.

mod queries;
mod rows;

use parabellum_app::villages::projection_repositories::MerchantMovementRepository;
use parabellum_app::villages::read_models::MerchantMovement;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::PgPool;

use crate::ProjectionDb;

use self::rows::DbMerchantMovementRow;

#[derive(Debug, Clone)]
pub struct PostgresMerchantMovementRepository {
    pool: PgPool,
}

impl PostgresMerchantMovementRepository {
    pub fn new(db: ProjectionDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }
}

#[async_trait::async_trait]
impl MerchantMovementRepository for PostgresMerchantMovementRepository {
    async fn list_active_for_village(
        &self,
        village_id: u32,
    ) -> Result<Vec<MerchantMovement>, ApplicationError> {
        let rows: Vec<DbMerchantMovementRow> = sqlx::query_as(queries::active_movements_sql())
            .bind(village_id as i32)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        rows.into_iter().map(TryInto::try_into).collect()
    }
}
