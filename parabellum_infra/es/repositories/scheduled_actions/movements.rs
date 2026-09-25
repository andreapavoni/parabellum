//! Scheduled movement lookup helpers.

use parabellum_types::errors::{ApplicationError, DbError};
use uuid::Uuid;

use parabellum_app::villages::{
    models::{ScheduledAction, ScheduledActionPayload, ScheduledActionType},
    projection_repositories::{ScheduledActionFilter, ScheduledActionOrder},
};

use super::{PostgresScheduledActionRepository, queries, rows::DbScheduledActionRow};

#[derive(Debug, Clone)]
pub(crate) struct PendingTroopArrivalAction {
    action: ScheduledAction,
}

impl From<DbScheduledActionRow> for PendingTroopArrivalAction {
    fn from(row: DbScheduledActionRow) -> Self {
        Self { action: row.into() }
    }
}

impl PendingTroopArrivalAction {
    /// Returns the scheduled action id used when updating or canceling the arrival.
    pub(crate) fn id(&self) -> Uuid {
        self.action.id
    }

    /// Returns the timestamp at which the troop arrival is due.
    pub(crate) fn execute_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.action.execute_at
    }

    /// Returns the timestamp at which the movement was scheduled.
    pub(crate) fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.action
            .created_at
            .unwrap_or_else(|| chrono::Utc::now())
    }

    /// Returns the decoded troop-arrival payload.
    pub(crate) fn payload(&self) -> Result<ScheduledActionPayload, ApplicationError> {
        self.action.payload()
    }

    /// Returns the movement id carried by this troop-arrival workflow.
    pub(crate) fn movement_id(&self) -> Result<Uuid, ApplicationError> {
        match self.payload()? {
            ScheduledActionPayload::AttackArrival { workflow } => Ok(workflow.movement_id),
            ScheduledActionPayload::ScoutArrival { workflow } => Ok(workflow.movement_id),
            ScheduledActionPayload::ReinforcementArrival { workflow } => Ok(workflow.movement_id),
            ScheduledActionPayload::SettlersArrival { workflow } => Ok(workflow.movement_id),
            _ => Err(ApplicationError::Unknown(
                "Scheduled action is not a troop arrival workflow".to_string(),
            )),
        }
    }
}

impl PostgresScheduledActionRepository {
    /// Returns the pending troop-arrival action for a movement.
    pub(crate) async fn find_pending_troop_arrival_by_movement_id(
        &self,
        movement_id: Uuid,
    ) -> Result<Option<PendingTroopArrivalAction>, ApplicationError> {
        let filter = pending_troop_arrival_filter()
            .movement(movement_id)
            .order_by(ScheduledActionOrder::CreatedAtDesc)
            .limit(1);

        queries::scheduled_action_row_query(filter)
            .build_query_as::<DbScheduledActionRow>()
            .fetch_optional(self.pool())
            .await
            .map(|row| row.map(Into::into))
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))
    }

    /// Returns pending troop arrivals that originated from a village.
    pub(crate) async fn list_pending_troop_arrivals_by_source_village(
        &self,
        village_id: u32,
    ) -> Result<Vec<PendingTroopArrivalAction>, ApplicationError> {
        let filter = pending_troop_arrival_filter()
            .source_or_village(village_id)
            .order_by(ScheduledActionOrder::CreatedAtAsc);

        queries::scheduled_action_row_query(filter)
            .build_query_as::<DbScheduledActionRow>()
            .fetch_all(self.pool())
            .await
            .map(|rows| rows.into_iter().map(Into::into).collect())
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))
    }
}

fn pending_troop_arrival_filter() -> ScheduledActionFilter {
    ScheduledActionFilter::new()
        .action_types(vec![
            ScheduledActionType::ReinforcementArrival,
            ScheduledActionType::SettlersArrival,
            ScheduledActionType::AttackArrival,
            ScheduledActionType::ScoutArrival,
        ])
        .pending()
}
