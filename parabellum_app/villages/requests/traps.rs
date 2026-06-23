//! Trap-building use-case inputs.
//!
//! These request types describe player intent for trapper actions. Use cases
//! load current app context and translate them into command/workflow intent.

use uuid::Uuid;

/// Player request to queue trap construction in a village.
#[derive(Debug, Clone)]
pub struct BuildTrapsRequest {
    /// Player expected to own the village.
    pub player_id: Uuid,
    /// Village containing the trapper.
    pub village_id: u32,
    /// Number of traps requested for construction.
    pub quantity: u32,
}
