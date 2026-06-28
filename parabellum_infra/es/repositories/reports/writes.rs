//! Write helpers for report projections.

use chrono::{DateTime, Utc};
use parabellum_app::villages::projection_repositories::ProjectedReport;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{Postgres, Transaction, types::Json};
use uuid::Uuid;

use super::PostgresReportRepository;

impl PostgresReportRepository {
    /// Stores a projected report and its audience rows inside an existing transaction.
    pub async fn add_projected_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        report: &ProjectedReport,
        audience_player_ids: &[Uuid],
    ) -> Result<Uuid, ApplicationError> {
        let report_id = report.id;

        sqlx::query(
            r#"
            INSERT INTO rm_reports (
                id, report_type, payload, actor_player_id, actor_village_id,
                target_player_id, target_village_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(report_id)
        .bind(&report.report_type)
        .bind(Json(&report.payload))
        .bind(report.actor_player_id)
        .bind(report.actor_village_id.map(|v| v as i32))
        .bind(report.target_player_id)
        .bind(report.target_village_id.map(|v| v as i32))
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        for player_id in audience_player_ids {
            sqlx::query(
                r#"
                INSERT INTO rm_report_reads (report_id, player_id, read_at)
                VALUES ($1, $2, NULL)
                "#,
            )
            .bind(report_id)
            .bind(player_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        }

        Ok(report_id)
    }

    /// Marks one report audience row as read inside an existing transaction.
    pub async fn mark_as_read_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        report_id: Uuid,
        player_id: Uuid,
        read_at: DateTime<Utc>,
    ) -> Result<bool, ApplicationError> {
        let result = sqlx::query(
            r#"
            UPDATE rm_report_reads
            SET read_at = $3
            WHERE report_id = $1 AND player_id = $2
            "#,
        )
        .bind(report_id)
        .bind(player_id)
        .bind(read_at)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(result.rows_affected() > 0)
    }

    /// Marks the latest unread report before `read_at` as read in a transaction.
    pub async fn mark_latest_unread_as_read_before_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        player_id: Uuid,
        read_at: DateTime<Utc>,
    ) -> Result<bool, ApplicationError> {
        let result = sqlx::query(
            r#"
            UPDATE rm_report_reads rr
            SET read_at = $2
            FROM rm_reports r
            WHERE rr.player_id = $1
              AND rr.read_at IS NULL
              AND rr.report_id = r.id
              AND r.created_at <= $2
              AND rr.report_id = (
                SELECT rr2.report_id
                FROM rm_report_reads rr2
                JOIN rm_reports r2 ON r2.id = rr2.report_id
                WHERE rr2.player_id = $1
                  AND rr2.read_at IS NULL
                  AND r2.created_at <= $2
                ORDER BY r2.created_at DESC, rr2.report_id DESC
                LIMIT 1
              )
            "#,
        )
        .bind(player_id)
        .bind(read_at)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(result.rows_affected() > 0)
    }

    pub(super) async fn mark_as_read_at(
        &self,
        report_id: Uuid,
        player_id: Uuid,
        read_at: DateTime<Utc>,
    ) -> Result<bool, ApplicationError> {
        let result = sqlx::query(
            r#"
            UPDATE rm_report_reads
            SET read_at = $3
            WHERE report_id = $1 AND player_id = $2
            "#,
        )
        .bind(report_id)
        .bind(player_id)
        .bind(read_at)
        .execute(self.pool())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(result.rows_affected() > 0)
    }
}
