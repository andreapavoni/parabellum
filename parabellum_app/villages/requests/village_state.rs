use uuid::Uuid;

/// Request full state for one village projection.
#[derive(Debug, Clone, Copy)]
pub struct GetVillageStateRequest {
    /// Village id to load.
    pub village_id: u32,
}

/// Request full village states owned by one player.
#[derive(Debug, Clone, Copy)]
pub struct ListPlayerVillageStatesRequest {
    /// Player id that owns the villages.
    pub player_id: Uuid,
}
