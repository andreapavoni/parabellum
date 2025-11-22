use serde::{Deserialize, Serialize};

/// Type of map flag/mark
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MapFlagType {
    /// Type 0: Player mark - tracks all villages owned by a specific player
    PlayerMark = 0,
    /// Type 1: Alliance mark - tracks all villages owned by all members of an alliance
    AllianceMark = 1,
    /// Type 2: Custom flag - static marker at specific map coordinates with custom text
    CustomFlag = 2,
}
