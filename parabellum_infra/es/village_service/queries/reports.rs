//! Report read composition for `VillageEsService`.
//!
//! Report lists and counters read from projected report rows. Marking a report
//! as read remains command-backed because it records a domain fact on the
//! village stream that owns the report.

use std::sync::Arc;

use mini_cqrs_es::{CqrsError, QueryRunner};
use parabellum_app::villages::VillageService;
use parabellum_app::villages::cqrs_queries::{
    CountUnreadReportsForPlayer, GetReportForPlayer, ListReportsForPlayer,
};
use parabellum_app::villages::models::ReportModel;
use parabellum_app::villages::projection_repositories::ReportRepository;

use crate::es::{PostgresReportRepository, village_cqrs_runtime};

use super::super::VillageEsService;

impl VillageEsService {
    /// Returns report rows visible to a player, ordered by the report query contract.
    pub async fn list_reports_for_player(
        &self,
        player_id: uuid::Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ReportModel>, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&ListReportsForPlayer {
                repository: Arc::new(PostgresReportRepository::new(crate::ProjectionDb::new(
                    self.pool.clone(),
                ))) as Arc<dyn ReportRepository>,
                player_id,
                offset,
                limit,
            })
            .await
    }

    /// Returns one report if it belongs to the player.
    pub async fn get_report_for_player(
        &self,
        report_id: uuid::Uuid,
        player_id: uuid::Uuid,
    ) -> Result<Option<ReportModel>, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&GetReportForPlayer {
                repository: Arc::new(PostgresReportRepository::new(crate::ProjectionDb::new(
                    self.pool.clone(),
                ))) as Arc<dyn ReportRepository>,
                report_id,
                player_id,
            })
            .await
    }

    /// Counts unread reports visible to the player.
    pub async fn count_unread_reports_for_player(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<i64, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&CountUnreadReportsForPlayer {
                repository: Arc::new(PostgresReportRepository::new(crate::ProjectionDb::new(
                    self.pool.clone(),
                ))) as Arc<dyn ReportRepository>,
                player_id,
            })
            .await
    }

    /// Marks a player-visible report as read through the owning village stream.
    pub async fn mark_report_as_read(
        &self,
        report_id: uuid::Uuid,
        player_id: uuid::Uuid,
    ) -> Result<(), CqrsError> {
        let report = self
            .get_report_for_player(report_id, player_id)
            .await?
            .ok_or_else(|| CqrsError::EventStore("report not found for player".to_string()))?;
        let village_id = report
            .actor_village_id
            .or(report.target_village_id)
            .ok_or_else(|| {
                CqrsError::EventStore("report has no village stream anchor".to_string())
            })?;

        let cqrs = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&cqrs);
        service
            .mark_report_read(
                village_id,
                &parabellum_app::villages::MarkReportRead {
                    report_id,
                    player_id,
                    read_at: chrono::Utc::now(),
                },
            )
            .await?;
        Ok(())
    }
}
