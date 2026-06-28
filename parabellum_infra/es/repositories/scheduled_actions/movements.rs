//! Scheduled movement lookup helpers.

use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::FromRow;
use uuid::Uuid;

use parabellum_app::villages::{
    models::ScheduledActionType,
    projection_repositories::{ScheduledActionFilter, ScheduledActionOrder},
};

use super::{PostgresScheduledActionRepository, queries};

#[derive(Debug, Clone, FromRow)]
pub(crate) struct PendingTroopArrivalActionRow {
    pub id: Uuid,
    pub execute_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub payload: serde_json::Value,
}

impl PostgresScheduledActionRepository {
    pub(crate) async fn find_pending_troop_arrival_by_movement_id(
        &self,
        movement_id: Uuid,
    ) -> Result<Option<PendingTroopArrivalActionRow>, ApplicationError> {
        let filter = pending_troop_arrival_filter()
            .movement(movement_id)
            .order_by(ScheduledActionOrder::CreatedAtDesc)
            .limit(1);

        let row = queries::scheduled_action_query(
            r#"
            SELECT id, execute_at, created_at, payload
            FROM rm_scheduled_actions
            "#,
            filter,
        )
        .build_query_as()
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(row)
    }

    pub(crate) async fn list_pending_troop_arrivals_by_source_village(
        &self,
        village_id: u32,
    ) -> Result<Vec<PendingTroopArrivalActionRow>, ApplicationError> {
        let filter = pending_troop_arrival_filter()
            .source_or_village(village_id)
            .order_by(ScheduledActionOrder::CreatedAtAsc);

        let rows = queries::scheduled_action_query(
            r#"
            SELECT id, execute_at, created_at, payload
            FROM rm_scheduled_actions
            "#,
            filter,
        )
        .build_query_as()
        .fetch_all(self.pool())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(rows)
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
