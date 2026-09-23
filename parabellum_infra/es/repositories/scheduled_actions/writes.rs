//! Scheduled-action write helpers.

use parabellum_app::villages::models::{ScheduledAction, ScheduledActionStatus};
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{Postgres, postgres::PgArguments, query::QueryAs};
use uuid::Uuid;

use super::{PostgresScheduledActionRepository, queries, rows::DbScheduledActionRow};

impl PostgresScheduledActionRepository {
    /// Serializes duplicate deliveries and rejects cancelled/terminal actions.
    pub(crate) async fn lock_processing_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        id: Uuid,
    ) -> Result<bool, mini_cqrs_es::CqrsError> {
        let processing = sqlx::query_scalar::<_, bool>(
            "SELECT status = 'processing' FROM rm_scheduled_actions WHERE id = $1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(mini_cqrs_es::CqrsError::domain_source)?;
        Ok(processing == Some(true))
    }

    /// A late error must never overwrite a completion committed by another attempt.
    pub(crate) async fn finish_processing(
        &self,
        id: Uuid,
        status: ScheduledActionStatus,
    ) -> Result<(), mini_cqrs_es::CqrsError> {
        if status == ScheduledActionStatus::Pending {
            // Persist the retry budget across worker restarts. Original workflow
            // timestamps remain in the payload; only dispatch is delayed.
            sqlx::query(
                r#"
                UPDATE rm_scheduled_actions
                SET status = CASE WHEN attempts >= 5 THEN 'failed'::scheduled_action_status
                                  ELSE 'pending'::scheduled_action_status END,
                    execute_at = GREATEST(execute_at, NOW() + make_interval(
                        secs => LEAST(60, power(2, LEAST(attempts, 6)))::double precision)),
                    updated_at = NOW()
                WHERE id = $1 AND status = 'processing'
            "#,
            )
            .bind(id)
            .execute(self.pool())
            .await
            .map_err(mini_cqrs_es::CqrsError::domain_source)?;
            return Ok(());
        }
        let mut query = queries::update_scheduled_action_status_query(id, status);
        query.push(" AND status = 'processing'");
        query
            .build()
            .execute(self.pool())
            .await
            .map_err(mini_cqrs_es::CqrsError::domain_source)?;
        Ok(())
    }

    pub(crate) async fn requeue_stale_processing(
        &self,
        updated_before_or_equal: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, ApplicationError> {
        let result = queries::requeue_stale_processing_query(updated_before_or_equal)
            .build()
            .execute(self.pool())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(result.rows_affected())
    }

    pub(crate) async fn add_direct(
        &self,
        action: &ScheduledAction,
    ) -> Result<(), ApplicationError> {
        queries::insert_scheduled_action_query(action)
            .build()
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
        queries::insert_scheduled_action_query(action)
            .build()
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
        queries::update_scheduled_action_status_query(id, status)
            .build()
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
        queries::update_scheduled_action_status_query(id, status)
            .build()
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

        let rows: Vec<DbScheduledActionRow> =
            claim_due_pending_actions_query(before_or_equal, limit)
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

/// Claims due pending actions for exclusive scheduler processing.
///
/// This stays as an explicit SQLx query because `FOR UPDATE SKIP LOCKED` is the
/// operational queue primitive, not a generic scheduled-action filter.
fn claim_due_pending_actions_query(
    before_or_equal: chrono::DateTime<chrono::Utc>,
    limit: i64,
) -> QueryAs<'static, Postgres, DbScheduledActionRow, PgArguments> {
    sqlx::query_as(
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
        SET status = 'processing', attempts = attempts + 1, updated_at = NOW()
        FROM due
        WHERE a.id = due.id
        RETURNING a.id, a.action_type, a.execute_at, a.payload, a.status, a.created_at
        "#,
    )
    .bind(before_or_equal)
    .bind(limit)
}
