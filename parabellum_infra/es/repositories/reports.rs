use parabellum_app::villages::models::ReportModel;
use parabellum_app::villages::repositories::{ProjectedReport, ReportRepository};
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::postgres::PgQueryResult;
use sqlx::{FromRow, PgPool, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PostgresReportRepository {
    pool: PgPool,
}

impl PostgresReportRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn add_projected_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        report: &ProjectedReport,
        audience_player_ids: &[Uuid],
    ) -> Result<Uuid, ApplicationError> {
        let report_id = report.id;

        sqlx::query(
            r#"
            INSERT INTO rm_reports (id, report_type, payload, actor_player_id, actor_village_id, target_player_id, target_village_id)
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
            let _result: PgQueryResult = sqlx::query(
                "INSERT INTO rm_report_reads (report_id, player_id, read_at) VALUES ($1, $2, NULL)",
            )
            .bind(report_id)
            .bind(player_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        }

        Ok(report_id)
    }

    pub async fn mark_as_read_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        report_id: Uuid,
        player_id: Uuid,
        read_at: chrono::DateTime<chrono::Utc>,
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

    pub async fn mark_latest_unread_as_read_before_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        player_id: Uuid,
        read_at: chrono::DateTime<chrono::Utc>,
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
}

#[derive(Debug, Clone, FromRow)]
struct DbReportRow {
    id: Uuid,
    report_type: String,
    payload: Json<serde_json::Value>,
    actor_player_id: Uuid,
    actor_village_id: Option<i32>,
    target_player_id: Option<Uuid>,
    target_village_id: Option<i32>,
    created_at: chrono::DateTime<chrono::Utc>,
    read_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl TryFrom<DbReportRow> for ReportModel {
    type Error = ApplicationError;

    fn try_from(row: DbReportRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            report_type: row.report_type,
            payload: serde_json::from_value(row.payload.0)?,
            actor_player_id: row.actor_player_id,
            actor_village_id: row.actor_village_id.map(|v| v as u32),
            target_player_id: row.target_player_id,
            target_village_id: row.target_village_id.map(|v| v as u32),
            created_at: row.created_at,
            read_at: row.read_at,
        })
    }
}

fn report_select_sql() -> &'static str {
    r#"
    SELECT
      r.id,
      r.report_type,
      r.payload,
      r.actor_player_id,
      r.actor_village_id,
      r.target_player_id,
      r.target_village_id,
      r.created_at,
      rr.read_at
    FROM rm_reports r
    JOIN rm_report_reads rr ON rr.report_id = r.id
    "#
}

#[async_trait::async_trait]
impl ReportRepository for PostgresReportRepository {
    async fn add_projected(
        &self,
        report: &ProjectedReport,
        audience_player_ids: &[Uuid],
    ) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        self.add_projected_in_tx(&mut tx, report, audience_player_ids)
            .await?;

        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn list_for_player(
        &self,
        player_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ReportModel>, ApplicationError> {
        let rows: Vec<DbReportRow> = sqlx::query_as(&format!(
            r#"
            {}
            WHERE rr.player_id = $1
            ORDER BY r.created_at DESC
            OFFSET $2
            LIMIT $3
            "#,
            report_select_sql()
        ))
        .bind(player_id)
        .bind(offset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn get_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<ReportModel>, ApplicationError> {
        let row: Option<DbReportRow> = sqlx::query_as(&format!(
            r#"
            {}
            WHERE r.id = $1 AND rr.player_id = $2
            "#,
            report_select_sql()
        ))
        .bind(report_id)
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        row.map(TryInto::try_into).transpose()
    }

    async fn count_unread_for_player(&self, player_id: Uuid) -> Result<i64, ApplicationError> {
        sqlx::query_scalar(
            r#"
            SELECT COUNT(*) as count
            FROM rm_report_reads
            WHERE player_id = $1 AND read_at IS NULL
            "#,
        )
        .bind(player_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))
    }

    async fn mark_as_read(&self, report_id: Uuid, player_id: Uuid) -> Result<(), ApplicationError> {
        sqlx::query!(
            r#"
            UPDATE rm_report_reads
            SET read_at = NOW()
            WHERE report_id = $1 AND player_id = $2
            "#,
            report_id,
            player_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }
}
