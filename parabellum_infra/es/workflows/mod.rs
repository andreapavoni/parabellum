//! Scheduled workflow orchestration.
//!
//! A scheduled action payload is an operational command envelope, not domain
//! history. These modules translate the strict `{ "workflow": { ... } }`
//! payloads into canonical village events that can be appended and replayed.
//!
//! Responsibilities:
//! - `parabellum_app` owns command/event contracts and aggregate outcomes.
//! - `parabellum_game` owns game rules and domain calculations.
//! - this module owns infrastructure orchestration: read-model lookups,
//!   repository access, cross-stream event grouping, and scheduler handoff.
//!
//! New workflows should keep that split intact. If a rule belongs to the game,
//! put it in the domain layer; if a fact shape belongs to the aggregate, expose
//! it through the app layer; keep SQL and read-model coordination here.

pub(crate) mod battles;
pub(crate) mod buildings;
pub(crate) mod foundation;
pub(crate) mod heroes;
pub(crate) mod merchants;
pub(crate) mod movements;
pub(crate) mod research;
pub(crate) mod training;

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    ScheduledAction, ScheduledActionPayload, ScheduledActionStatus,
};
use uuid::Uuid;

/// Events produced by one scheduled workflow execution.
///
/// The tuple key is the village aggregate stream id that receives the event.
/// Multi-stream workflows use multiple keys and are appended atomically by the
/// village ES service.
#[derive(Debug, Default)]
pub(crate) struct WorkflowEvents {
    events: Vec<(u32, VillageEvent)>,
}

impl WorkflowEvents {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn one(village_id: u32, event: VillageEvent) -> Self {
        Self {
            events: vec![(village_id, event)],
        }
    }

    pub(crate) fn from_events(events: Vec<(u32, VillageEvent)>) -> Self {
        Self { events }
    }

    pub(crate) fn push(&mut self, village_id: u32, event: VillageEvent) {
        self.events.push((village_id, event));
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub(crate) fn into_inner(self) -> Vec<(u32, VillageEvent)> {
        self.events
    }
}

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
