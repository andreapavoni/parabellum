use parabellum_app::villages::models::ReportModel;
use parabellum_app::villages::repositories::ReportReadModelRepository;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{PgPool, types::Json};
use uuid::Uuid;

pub struct NewProjectedReport {
    pub report_type: String,
    pub payload: serde_json::Value,
    pub actor_player_id: Uuid,
    pub actor_village_id: Option<u32>,
    pub target_player_id: Option<Uuid>,
    pub target_village_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct PostgresReportReadModelRepository {
    pool: PgPool,
}

impl PostgresReportReadModelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn add(
        &self,
        report: &NewProjectedReport,
        audience_player_ids: &[Uuid],
    ) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        let report_id = Uuid::new_v4();

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
        .execute(&mut *tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        for player_id in audience_player_ids {
            sqlx::query(
                "INSERT INTO rm_report_reads (report_id, player_id, read_at) VALUES ($1, $2, NULL)",
            )
            .bind(report_id)
            .bind(player_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        }

        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ReportReadModelRepository for PostgresReportReadModelRepository {
    async fn list_for_player(
        &self,
        player_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ReportModel>, ApplicationError> {
        let rows = sqlx::query!(
            r#"
            SELECT
              r.id,
              r.report_type,
              r.payload as "payload!: Json<serde_json::Value>",
              r.actor_player_id,
              r.actor_village_id,
              r.target_player_id,
              r.target_village_id,
              r.created_at,
              rr.read_at
            FROM rm_reports r
            JOIN rm_report_reads rr ON rr.report_id = r.id
            WHERE rr.player_id = $1
            ORDER BY r.created_at DESC
            LIMIT $2
            "#,
            player_id,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            records.push(ReportModel {
                id: row.id,
                report_type: row.report_type,
                payload: serde_json::from_value(row.payload.0)?,
                actor_player_id: row.actor_player_id,
                actor_village_id: row.actor_village_id.map(|v| v as u32),
                target_player_id: row.target_player_id,
                target_village_id: row.target_village_id.map(|v| v as u32),
                created_at: row.created_at,
                read_at: row.read_at,
            });
        }
        Ok(records)
    }

    async fn get_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<ReportModel>, ApplicationError> {
        let row = sqlx::query!(
            r#"
            SELECT
              r.id,
              r.report_type,
              r.payload as "payload!: Json<serde_json::Value>",
              r.actor_player_id,
              r.actor_village_id,
              r.target_player_id,
              r.target_village_id,
              r.created_at,
              rr.read_at
            FROM rm_reports r
            JOIN rm_report_reads rr ON rr.report_id = r.id
            WHERE r.id = $1 AND rr.player_id = $2
            "#,
            report_id,
            player_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(row
            .map(|row| -> Result<ReportModel, ApplicationError> {
                Ok(ReportModel {
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
            })
            .transpose()?)
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
