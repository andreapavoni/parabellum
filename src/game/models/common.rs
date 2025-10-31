use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::game::models::Tribe;

#[derive(Debug, Clone)]
pub struct Cost {
    pub resources: ResourceGroup,
    pub upkeep: u32,
    pub build_time: u32,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ResourceGroup(pub u32, pub u32, pub u32, pub u32);

impl ResourceGroup {
    pub const fn new(lumber: u32, clay: u32, iron: u32, crop: u32) -> Self {
        Self(lumber, clay, iron, crop)
    }

    pub fn total(&self) -> u32 {
        self.0 + self.1 + self.2 + self.3
    }
}

pub type SmithyUpgrades = [u8; 10];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
    pub tribe: Tribe,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_group_total() {
        let rg = ResourceGroup::new(100, 200, 300, 400);
        assert_eq!(rg.total(), 1000);

        let rg_zero = ResourceGroup::new(0, 0, 0, 0);
        assert_eq!(rg_zero.total(), 0);
    }
}
