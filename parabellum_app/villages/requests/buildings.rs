//! Building lifecycle use-case inputs.
//!
//! These request types describe player intent for scheduling or canceling
//! building construction. Use cases translate them into aggregate commands.

use parabellum_types::buildings::BuildingName;
use uuid::Uuid;

/// Player request to construct a new building on an empty village slot.
#[derive(Debug, Clone)]
pub struct AddBuildingRequest {
    /// Player expected to own the village.
    pub player_id: Uuid,
    /// Village where construction should be scheduled.
    pub village_id: u32,
    /// Target building slot.
    pub slot_id: u8,
    /// Building type to construct.
    pub building_name: BuildingName,
}

/// Player request to upgrade an existing building slot.
#[derive(Debug, Clone)]
pub struct UpgradeBuildingRequest {
    /// Player expected to own the village.
    pub player_id: Uuid,
    /// Village where the upgrade should be scheduled.
    pub village_id: u32,
    /// Target building slot.
    pub slot_id: u8,
}

/// Player request to downgrade an existing building slot.
#[derive(Debug, Clone)]
pub struct DowngradeBuildingRequest {
    /// Player expected to own the village.
    pub player_id: Uuid,
    /// Village where the downgrade should be scheduled.
    pub village_id: u32,
    /// Target building slot.
    pub slot_id: u8,
}

/// Player request to cancel a queued building construction action.
#[derive(Debug, Clone)]
pub struct CancelBuildingConstructionRequest {
    /// Player expected to own the village.
    pub player_id: Uuid,
    /// Village whose building queue contains the action.
    pub village_id: u32,
    /// Scheduled action to cancel.
    pub action_id: Uuid,
}
