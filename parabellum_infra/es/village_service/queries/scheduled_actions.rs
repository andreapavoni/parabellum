//! Scheduled-action read helpers for `VillageEsService`.
//!
//! These methods expose queue and status-count projections used by UI and
//! command validation flows.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::cqrs_queries::ScheduledActionStatusCounts;
use parabellum_app::villages::models::{ScheduledActionStatus, ScheduledActionType};
use parabellum_app::villages::projection_repositories::ScheduledActionRepository;
use parabellum_app::villages::read_models::VillageQueues;

use crate::es::PostgresScheduledActionRepository;

use super::super::VillageEsService;

impl VillageEsService {
    /// Returns active scheduled queues for one village.
    pub async fn get_village_queues(&self, village_id: u32) -> Result<VillageQueues, CqrsError> {
        PostgresScheduledActionRepository::new(crate::ProjectionDb::new(self.pool.clone()))
            .list_village_queues(village_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    /// Returns scheduled-action status counts for one village and action type.
    pub async fn get_village_scheduled_action_status_counts(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
        status_filter: Option<ScheduledActionStatus>,
    ) -> Result<ScheduledActionStatusCounts, CqrsError> {
        let repo =
            PostgresScheduledActionRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        repo.count_by_village_and_type(village_id, action_type, status_filter)
            .await
            .map_err(CqrsError::domain_source)
    }

    /// Returns one scheduled-action status count for one village and action type.
    pub async fn get_village_scheduled_action_status_count(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
        status: ScheduledActionStatus,
    ) -> Result<usize, CqrsError> {
        let counts = self
            .get_village_scheduled_action_status_counts(village_id, action_type, Some(status))
            .await?;
        Ok(match status {
            ScheduledActionStatus::Pending => counts.pending,
            ScheduledActionStatus::Processing => counts.processing,
            ScheduledActionStatus::Completed => counts.completed,
            ScheduledActionStatus::Failed => counts.failed,
            ScheduledActionStatus::Canceled => counts.canceled,
        })
    }
}
