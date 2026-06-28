//! Scheduled hero lifecycle lookup helpers.

use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::FromRow;
use uuid::Uuid;

use parabellum_app::villages::{
    models::ScheduledActionType,
    projection_repositories::{ScheduledActionFilter, ScheduledActionOrder},
};

use super::{PostgresScheduledActionRepository, queries};

#[derive(Debug, Clone, FromRow)]
pub(crate) struct PendingHeroRevivalAction {
    /// Timestamp at which the pending hero revival action is due.
    pub execute_at: chrono::DateTime<chrono::Utc>,
}

impl PostgresScheduledActionRepository {
    /// Returns the earliest pending hero revival action for a player.
    pub(crate) async fn pending_hero_revival_for_player(
        &self,
        player_id: Uuid,
    ) -> Result<Option<PendingHeroRevivalAction>, ApplicationError> {
        let filter = ScheduledActionFilter::new()
            .action_type(ScheduledActionType::HeroRevival)
            .pending()
            .player(player_id)
            .order_by(ScheduledActionOrder::ExecuteAtAsc)
            .limit(1);

        let action = queries::scheduled_action_query(
            r#"
            SELECT execute_at
            FROM rm_scheduled_actions
            "#,
            filter,
        )
        .build_query_as()
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(action)
    }
}
