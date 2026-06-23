//! Village activity query request types.
//!
//! These DTOs describe app-facing reads for queue and movement activity.

/// Request to load all queue summaries for a village.
#[derive(Debug, Clone, Copy)]
pub struct GetVillageQueuesRequest {
    /// Village whose queues should be loaded.
    pub village_id: u32,
}

/// Request to load troop movements for a village.
#[derive(Debug, Clone, Copy)]
pub struct GetVillageTroopMovementsRequest {
    /// Village whose troop movements should be loaded.
    pub village_id: u32,
}

/// Request to list cancelable outgoing troop movement ids for a village.
#[derive(Debug, Clone, Copy)]
pub struct ListCancelableOutgoingMovementIdsRequest {
    /// Village whose outgoing movements should be inspected.
    pub village_id: u32,
}
