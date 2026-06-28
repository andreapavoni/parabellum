//! Postgres scheduled-action repository.
//!
//! Scheduled actions are operational projection rows, not canonical domain
//! history. This module keeps the public repository type stable while splitting
//! SQL rows, query construction, writes, and specialized read helpers by
//! concern.

mod heroes;
mod movements;
pub(crate) mod queries;
mod queues;
mod rows;
mod writes;

use parabellum_app::villages::cqrs_queries::ScheduledActionStatusCounts;
use parabellum_app::villages::models::{
    ScheduledAction, ScheduledActionStatus, ScheduledActionType,
};
use parabellum_app::villages::projection_repositories::{
    ScheduledActionFilter, ScheduledActionRepository,
};
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::PgPool;
use uuid::Uuid;

use self::rows::DbScheduledActionRow;
use crate::ProjectionDb;

#[derive(Debug, Clone)]
pub struct PostgresScheduledActionRepository {
    pool: PgPool,
}

impl PostgresScheduledActionRepository {
    pub fn new(db: ProjectionDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    fn pool(&self) -> &PgPool {
        &self.pool
    }

    async fn list_actions_by_filter(
        &self,
        filter: ScheduledActionFilter,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        let rows: Vec<DbScheduledActionRow> = queries::scheduled_action_row_query(filter)
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait::async_trait]
impl ScheduledActionRepository for PostgresScheduledActionRepository {
    async fn add(&self, action: &ScheduledAction) -> Result<(), ApplicationError> {
        self.add_direct(action).await
    }

    async fn get_by_id(&self, id: Uuid) -> Result<ScheduledAction, ApplicationError> {
        let row: DbScheduledActionRow = sqlx::query_as(
            r#"
            SELECT id, action_type, execute_at, payload, status, created_at
            FROM rm_scheduled_actions
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(row.into())
    }

    async fn take_due_pending(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        self.claim_due_pending_actions(before_or_equal, limit).await
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: ScheduledActionStatus,
    ) -> Result<(), ApplicationError> {
        self.update_status_by_id(id, status).await
    }

    async fn list_actions(
        &self,
        filter: ScheduledActionFilter,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        self.list_actions_by_filter(filter).await
    }

    async fn count_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
        status_filter: Option<ScheduledActionStatus>,
    ) -> Result<ScheduledActionStatusCounts, ApplicationError> {
        let mut filter = ScheduledActionFilter::new()
            .village(village_id)
            .action_type(action_type);
        if let Some(status) = status_filter {
            filter = filter.statuses(vec![status]);
        }

        let row: (i64, i64, i64, i64, i64) = queries::scheduled_action_aggregate_query(
            r#"
            SELECT
              COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0)::bigint AS pending_count,
              COALESCE(SUM(CASE WHEN status = 'processing' THEN 1 ELSE 0 END), 0)::bigint AS processing_count,
              COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0)::bigint AS completed_count,
              COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0)::bigint AS failed_count,
              COALESCE(SUM(CASE WHEN status = 'canceled' THEN 1 ELSE 0 END), 0)::bigint AS canceled_count
            FROM rm_scheduled_actions
            "#,
            filter,
        )
        .build_query_as()
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(ScheduledActionStatusCounts {
            pending: row.0 as usize,
            processing: row.1 as usize,
            completed: row.2 as usize,
            failed: row.3 as usize,
            canceled: row.4 as usize,
        })
    }
}
