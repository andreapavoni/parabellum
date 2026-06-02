pub(crate) mod buildings;
pub(crate) mod heroes;
pub(crate) mod merchants;
pub(crate) mod movements;
pub(crate) mod research;
pub(crate) mod training;

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::models::{
    ScheduledAction, ScheduledActionPayload, ScheduledActionStatus,
};
use uuid::Uuid;

pub(crate) fn scheduled_action(
    action_id: Uuid,
    execute_at: chrono::DateTime<chrono::Utc>,
    payload: ScheduledActionPayload,
) -> Result<ScheduledAction, CqrsError> {
    Ok(ScheduledAction {
        id: action_id,
        action_type: payload.action_type(),
        execute_at,
        payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
        status: ScheduledActionStatus::Pending,
    })
}
