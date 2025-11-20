use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllianceBonusType {
    Training = 1,
    Armor = 2,
    CropProduction = 3,
    Trade = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceBonusUpgradeQueue {
    pub id: i32,
    pub aid: i32,
    pub type_: i16,
    pub time: i32,
}

impl AllianceBonusType {
    pub fn from_i16(val: i16) -> Option<Self> {
        match val {
            1 => Some(Self::Training),
            2 => Some(Self::Armor),
            3 => Some(Self::CropProduction),
            4 => Some(Self::Trade),
            _ => None,
        }
    }
}
