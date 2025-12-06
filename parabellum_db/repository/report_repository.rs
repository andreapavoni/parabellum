use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::{NewReport, ReportAudience, ReportRecord, ReportRepository};
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
};

#[derive(Clone)]
pub struct PostgresReportRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresReportRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> ReportRepository for PostgresReportRepository<'a> {
    async fn add(
        &self,
        report: &NewReport,
        audiences: &[ReportAudience],
    ) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let report_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO reports (id, report_type, payload, actor_player_id, actor_village_id, target_player_id, target_village_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            report_id,
            report.report_type,
            serde_json::to_value(&report.payload)?,
            report.actor_player_id,
            report.actor_village_id.map(|id| id as i32),
            report.target_player_id,
            report.target_village_id.map(|id| id as i32)
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        for audience in audiences {
            sqlx::query!(
                r#"
                INSERT INTO report_reads (report_id, player_id, read_at)
                VALUES ($1, $2, $3)
                "#,
                report_id,
                audience.player_id,
                audience.read_at
            )
            .execute(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        }

        Ok(())
    }

    async fn list_for_player(
        &self,
        player_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ReportRecord>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT r.id, r.report_type, r.payload, r.actor_player_id, r.actor_village_id, r.target_player_id, r.target_village_id, r.created_at, rr.read_at
            FROM reports r
            JOIN report_reads rr ON rr.report_id = r.id
            WHERE rr.player_id = $1
            ORDER BY r.created_at DESC
            LIMIT $2
            "#,
            player_id,
            limit
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(ReportRecord {
                id: row.id,
                report_type: row.report_type,
                payload: serde_json::from_value(row.payload)?,
                actor_player_id: row.actor_player_id,
                actor_village_id: row.actor_village_id.map(|id| id as u32),
                target_player_id: row.target_player_id,
                target_village_id: row.target_village_id.map(|id| id as u32),
                created_at: row.created_at,
                read_at: row.read_at,
            });
        }

        Ok(records)
    }

    async fn mark_as_read(&self, report_id: Uuid, player_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        sqlx::query!(
            r#"
            UPDATE report_reads
            SET read_at = NOW()
            WHERE report_id = $1 AND player_id = $2
            "#,
            report_id,
            player_id
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }
}
