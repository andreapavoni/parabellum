//! Scheduled-action write helpers.

use parabellum_app::villages::models::{ScheduledAction, ScheduledActionStatus};
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{Postgres, postgres::PgArguments, query::Query, types::Json};
use uuid::Uuid;

use super::{
    PostgresScheduledActionRepository,
    rows::{DbScheduledActionRow, DbScheduledActionStatus, DbScheduledActionType},
};

impl PostgresScheduledActionRepository {
    pub(crate) async fn requeue_stale_processing(
        &self,
        updated_before_or_equal: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, ApplicationError> {
        let result = sqlx::query(
            r#"
            UPDATE rm_scheduled_actions
            SET status = 'pending', updated_at = NOW()
            WHERE status = 'processing'
              AND updated_at <= $1
            "#,
        )
        .bind(updated_before_or_equal)
        .execute(self.pool())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(result.rows_affected())
    }

    pub(crate) async fn add_direct(
        &self,
        action: &ScheduledAction,
    ) -> Result<(), ApplicationError> {
        insert_action_query(action)
            .execute(self.pool())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub(crate) async fn add_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        action: &ScheduledAction,
    ) -> Result<(), ApplicationError> {
        insert_action_query(action)
            .execute(&mut **tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub(crate) async fn update_status_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        status: ScheduledActionStatus,
    ) -> Result<(), ApplicationError> {
        update_status_query(id, status)
            .execute(&mut **tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub(super) async fn update_status_by_id(
        &self,
        id: Uuid,
        status: ScheduledActionStatus,
    ) -> Result<(), ApplicationError> {
        update_status_query(id, status)
            .execute(self.pool())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub(super) async fn claim_due_pending_actions(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let rows: Vec<DbScheduledActionRow> = sqlx::query_as(
            r#"
            WITH due AS (
                SELECT id
                FROM rm_scheduled_actions
                WHERE status = 'pending' AND execute_at <= $1
                ORDER BY execute_at ASC
                LIMIT $2
                FOR UPDATE SKIP LOCKED
            )
            UPDATE rm_scheduled_actions a
            SET status = 'processing', updated_at = NOW()
            FROM due
            WHERE a.id = due.id
            RETURNING a.id, a.action_type, a.execute_at, a.payload, a.status, a.created_at
            "#,
        )
        .bind(before_or_equal)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

fn insert_action_query<'a>(action: &'a ScheduledAction) -> Query<'a, Postgres, PgArguments> {
    sqlx::query(
        r#"
        INSERT INTO rm_scheduled_actions (id, action_type, execute_at, payload, status)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(action.id)
    .bind(DbScheduledActionType::from(action.action_type))
    .bind(action.execute_at)
    .bind(Json(&action.payload))
    .bind(DbScheduledActionStatus::from(action.status))
}

fn update_status_query(
    id: Uuid,
    status: ScheduledActionStatus,
) -> Query<'static, Postgres, PgArguments> {
    sqlx::query(
        r#"
        UPDATE rm_scheduled_actions
        SET status = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(DbScheduledActionStatus::from(status))
}
