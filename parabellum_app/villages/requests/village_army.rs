//! Village army query request types.
//!
//! These DTOs describe app-facing reads for village army state.

/// Request to load the full army state view for a village.
#[derive(Debug, Clone, Copy)]
pub struct GetVillageArmyStateViewRequest {
    /// Village whose army state should be loaded.
    pub village_id: u32,
}
