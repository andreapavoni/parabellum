use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub enum Tribe {
    Roman,
    Gaul,
    Teuton,
    Natar,
    Nature,
}

#[derive(Debug, Clone)]
pub struct Cost {
    pub resources: ResourceGroup,
    pub upkeep: u32,
    pub build_time: u32,
}

#[derive(Debug, Clone)]
pub struct ResourceGroup(u32, u32, u32, u32);

impl ResourceGroup {
    pub const fn new(lumber: u32, clay: u32, iron: u32, crop: u32) -> Self {
        Self(lumber, clay, iron, crop)
    }
}

pub type SmithyUpgrades = [u8; 10];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
    pub tribe: Tribe,
}
