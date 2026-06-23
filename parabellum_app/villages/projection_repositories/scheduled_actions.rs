//! Scheduled-action projection repository contracts.

use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::{
    cqrs_queries::ScheduledActionStatusCounts,
    models::{ScheduledAction, ScheduledActionStatus, ScheduledActionType},
};

/// Village side used when filtering scheduled actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduledActionVillageFilter {
    Source(u32),
    Target(u32),
}

/// Filter for listing scheduled action projections.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScheduledActionListFilter {
    pub action_type: Option<ScheduledActionType>,
    pub village: Option<ScheduledActionVillageFilter>,
    pub statuses: Option<Vec<ScheduledActionStatus>>,
}

impl ScheduledActionListFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn action_type(mut self, action_type: ScheduledActionType) -> Self {
        self.action_type = Some(action_type);
        self
    }

    pub fn source_village(mut self, village_id: u32) -> Self {
        self.village = Some(ScheduledActionVillageFilter::Source(village_id));
        self
    }

    pub fn target_village(mut self, target_village_id: u32) -> Self {
        self.village = Some(ScheduledActionVillageFilter::Target(target_village_id));
        self
    }

    pub fn statuses(mut self, statuses: Vec<ScheduledActionStatus>) -> Self {
        self.statuses = Some(statuses);
        self
    }

    pub fn active(self) -> Self {
        self.statuses(vec![
            ScheduledActionStatus::Pending,
            ScheduledActionStatus::Processing,
        ])
    }
}

/// Persistence boundary for scheduled action projections.
#[async_trait::async_trait]
pub trait ScheduledActionRepository: Send + Sync {
    async fn add(&self, action: &ScheduledAction) -> Result<(), ApplicationError>;

    async fn get_by_id(&self, id: Uuid) -> Result<ScheduledAction, ApplicationError>;

    async fn take_due_pending(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<ScheduledAction>, ApplicationError>;

    async fn update_status(
        &self,
        id: Uuid,
        status: ScheduledActionStatus,
    ) -> Result<(), ApplicationError>;

    async fn list_actions(
        &self,
        filter: ScheduledActionListFilter,
    ) -> Result<Vec<ScheduledAction>, ApplicationError>;

    async fn list_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        self.list_actions(
            ScheduledActionListFilter::new()
                .source_village(village_id)
                .action_type(action_type),
        )
        .await
    }

    async fn list_by_target_village_and_type(
        &self,
        target_village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        self.list_actions(
            ScheduledActionListFilter::new()
                .target_village(target_village_id)
                .action_type(action_type),
        )
        .await
    }

    async fn list_active_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        self.list_actions(
            ScheduledActionListFilter::new()
                .source_village(village_id)
                .action_type(action_type)
                .active(),
        )
        .await
    }

    async fn list_active_by_target_village_and_type(
        &self,
        target_village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        self.list_actions(
            ScheduledActionListFilter::new()
                .target_village(target_village_id)
                .action_type(action_type)
                .active(),
        )
        .await
    }

    async fn count_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
        status_filter: Option<ScheduledActionStatus>,
    ) -> Result<ScheduledActionStatusCounts, ApplicationError>;
}
