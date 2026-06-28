//! Postgres implementation of projected report repositories.

mod queries;
mod rows;
mod writes;

use parabellum_app::villages::models::ReportModel;
use parabellum_app::villages::projection_repositories::{
    ProjectedReport, ReportFilter, ReportRepository,
};
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ProjectionDb;

use rows::DbReportRow;

/// Postgres-backed repository for report projections and audience read state.
#[derive(Debug, Clone)]
pub struct PostgresReportRepository {
    pool: PgPool,
}

impl PostgresReportRepository {
    /// Creates a report repository backed by the projection database.
    pub fn new(db: ProjectionDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub(super) fn pool(&self) -> &PgPool {
        &self.pool
    }
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

    async fn list_reports(
        &self,
        filter: ReportFilter,
    ) -> Result<Vec<ReportModel>, ApplicationError> {
        let rows: Vec<DbReportRow> = queries::report_list_query(filter)
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn find_report(
        &self,
        filter: ReportFilter,
    ) -> Result<Option<ReportModel>, ApplicationError> {
        let row: Option<DbReportRow> = queries::report_list_query(filter.page(0, 1))
            .build_query_as()
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        row.map(TryInto::try_into).transpose()
    }

    async fn count_reports(&self, filter: ReportFilter) -> Result<i64, ApplicationError> {
        queries::report_count_query(filter)
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))
    }

    async fn mark_as_read(&self, report_id: Uuid, player_id: Uuid) -> Result<(), ApplicationError> {
        self.mark_as_read_at(report_id, player_id, chrono::Utc::now())
            .await?;
        Ok(())
    }
}
