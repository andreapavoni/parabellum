//! Read/context port for movement-control use cases.
//!
//! Movement control requires current workflow context and source-village
//! ownership before building command intent.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parabellum_game::models::army::Army;
use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::models::VillageModel;

/// Current workflow context for an outgoing troop movement that may be canceled.
#[derive(Debug, Clone)]
pub struct CancelTroopMovementContext {
    /// Movement identifier shared by the outbound workflow.
    pub movement_id: Uuid,
    /// Scheduled arrival action to cancel.
    pub arrival_action_id: Uuid,
    /// Moving army id.
    pub army_id: Uuid,
    /// Player that dispatched the movement.
    pub player_id: Uuid,
    /// Source village that dispatched the movement.
    pub source_village_id: u32,
    /// Target village or map field.
    pub target_village_id: u32,
    /// Moving army payload.
    pub army: Army,
    /// Time when the outbound workflow was created.
    pub sent_at: DateTime<Utc>,
    /// Planned arrival time.
    pub arrives_at: DateTime<Utc>,
}

/// Loads read-model context required by movement-control use cases.
#[async_trait]
pub trait MovementControlReadPort: Send + Sync {
    /// Returns cancelable movement workflow context for a movement id.
    async fn get_cancel_troop_movement_context(
        &self,
        movement_id: Uuid,
    ) -> Result<CancelTroopMovementContext, ApplicationError>;

    /// Returns the current source village read model for ownership checks.
    async fn get_movement_control_village(
        &self,
        village_id: u32,
    ) -> Result<VillageModel, ApplicationError>;
}
