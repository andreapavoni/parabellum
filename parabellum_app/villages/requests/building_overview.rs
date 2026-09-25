//! Building preview query input.
use uuid::Uuid;

/// Selects one building slot owned by the requesting player.
#[derive(Debug, Clone, Copy)]
pub struct GetBuildingOverviewRequest {
    pub player_id: Uuid,
    pub village_id: u32,
    pub slot_id: u8,
}
