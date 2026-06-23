//! Movement-control use-case inputs.
//!
//! These request types describe player intent for controlling already-created
//! movements. Use cases load workflow context and translate intent into
//! command/workflow updates.

use uuid::Uuid;

/// Player request to cancel an outgoing troop movement during the cancel window.
#[derive(Debug, Clone)]
pub struct CancelTroopMovementRequest {
    /// Player expected to own the source village.
    pub player_id: Uuid,
    /// Source village that dispatched the movement.
    pub village_id: u32,
    /// Movement to cancel.
    pub movement_id: Uuid,
}
