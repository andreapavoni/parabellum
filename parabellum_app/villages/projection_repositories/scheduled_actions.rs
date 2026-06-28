//! Scheduled-action projection repository contracts.

use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::{
    cqrs_queries::ScheduledActionStatusCounts,
    models::{ScheduledAction, ScheduledActionStatus, ScheduledActionType},
};

/// Workflow identity predicate used when filtering scheduled actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduledActionWorkflowFilter {
    /// Matches `workflow.village_id`.
    Village(u32),
    /// Matches `workflow.source_village_id`.
    SourceVillage(u32),
    /// Matches `workflow.target_village_id`.
    TargetVillage(u32),
    /// Matches either `workflow.source_village_id` or `workflow.village_id`.
    SourceOrVillage(u32),
    /// Matches `workflow.player_id`.
    Player(Uuid),
    /// Matches `workflow.movement_id`.
    Movement(Uuid),
}

/// Ordering used for scheduled-action projection queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduledActionOrder {
    /// Sort by `execute_at ASC, created_at ASC`.
    ExecuteAtAsc,
    /// Sort by `created_at ASC`.
    CreatedAtAsc,
    /// Sort by `created_at DESC`.
    CreatedAtDesc,
}

impl Default for ScheduledActionOrder {
    fn default() -> Self {
        Self::ExecuteAtAsc
    }
}

/// Filter for scheduled action projections.
///
/// The filter is an app-level contract: it names scheduled-action concepts
/// such as action type, status, player, village, and movement. Infrastructure
/// implementations decide how those concepts map to SQL columns or projection
/// payload fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScheduledActionFilter {
    pub action_types: Option<Vec<ScheduledActionType>>,
    pub statuses: Option<Vec<ScheduledActionStatus>>,
    pub workflow_filters: Vec<ScheduledActionWorkflowFilter>,
    pub order: ScheduledActionOrder,
    pub limit: Option<i64>,
}

impl ScheduledActionFilter {
    /// Creates an empty scheduled-action filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restricts the query to one action type.
    pub fn action_type(mut self, action_type: ScheduledActionType) -> Self {
        self.action_types = Some(vec![action_type]);
        self
    }

    /// Restricts the query to any of the provided action types.
    pub fn action_types(mut self, action_types: Vec<ScheduledActionType>) -> Self {
        self.action_types = Some(action_types);
        self
    }

    /// Restricts the query to actions whose workflow belongs to a village.
    pub fn village(mut self, village_id: u32) -> Self {
        self.workflow_filters
            .push(ScheduledActionWorkflowFilter::Village(village_id));
        self
    }

    /// Restricts the query to actions whose workflow source village matches.
    pub fn source_village(mut self, source_village_id: u32) -> Self {
        self.workflow_filters
            .push(ScheduledActionWorkflowFilter::SourceVillage(
                source_village_id,
            ));
        self
    }

    /// Restricts the query to actions whose workflow target village matches.
    pub fn target_village(mut self, target_village_id: u32) -> Self {
        self.workflow_filters
            .push(ScheduledActionWorkflowFilter::TargetVillage(
                target_village_id,
            ));
        self
    }

    /// Restricts the query to actions whose workflow source or owning village matches.
    pub fn source_or_village(mut self, village_id: u32) -> Self {
        self.workflow_filters
            .push(ScheduledActionWorkflowFilter::SourceOrVillage(village_id));
        self
    }

    /// Restricts the query to actions whose workflow player matches.
    pub fn player(mut self, player_id: Uuid) -> Self {
        self.workflow_filters
            .push(ScheduledActionWorkflowFilter::Player(player_id));
        self
    }

    /// Restricts the query to actions whose workflow movement matches.
    pub fn movement(mut self, movement_id: Uuid) -> Self {
        self.workflow_filters
            .push(ScheduledActionWorkflowFilter::Movement(movement_id));
        self
    }

    /// Restricts the query to any of the provided statuses.
    pub fn statuses(mut self, statuses: Vec<ScheduledActionStatus>) -> Self {
        self.statuses = Some(statuses);
        self
    }

    /// Restricts the query to pending or processing actions.
    pub fn active(self) -> Self {
        self.statuses(vec![
            ScheduledActionStatus::Pending,
            ScheduledActionStatus::Processing,
        ])
    }

    /// Restricts the query to pending actions.
    pub fn pending(self) -> Self {
        self.statuses(vec![ScheduledActionStatus::Pending])
    }

    /// Sets the result ordering.
    pub fn order_by(mut self, order: ScheduledActionOrder) -> Self {
        self.order = order;
        self
    }

    /// Limits the number of returned rows.
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
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
        filter: ScheduledActionFilter,
    ) -> Result<Vec<ScheduledAction>, ApplicationError>;

    async fn list_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        self.list_actions(
            ScheduledActionFilter::new()
                .village(village_id)
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
            ScheduledActionFilter::new()
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
            ScheduledActionFilter::new()
                .village(village_id)
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
            ScheduledActionFilter::new()
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
