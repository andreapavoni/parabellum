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

impl MapFlagType {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            0 => Some(MapFlagType::PlayerMark),
            1 => Some(MapFlagType::AllianceMark),
            2 => Some(MapFlagType::CustomFlag),
            _ => None,
        }
    }

    pub fn as_i16(self) -> i16 {
        self as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_flag_type_conversion() {
        assert_eq!(MapFlagType::from_i16(0).unwrap(), MapFlagType::PlayerMark);
        assert_eq!(MapFlagType::from_i16(1).unwrap(), MapFlagType::AllianceMark);
        assert_eq!(MapFlagType::from_i16(2).unwrap(), MapFlagType::CustomFlag);
        assert!(MapFlagType::from_i16(3).is_none());
    }
}
