//! Village profile use-case inputs.
//!
//! These request types describe player intent for changing village metadata.

use uuid::Uuid;

/// Player request to rename an owned village.
#[derive(Debug, Clone)]
pub struct RenameVillageRequest {
    /// Player expected to own the village.
    pub player_id: Uuid,
    /// Village to rename.
    pub village_id: u32,
    /// Requested village name.
    pub village_name: String,
}
