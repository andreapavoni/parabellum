#[derive(Debug, Clone, Hash, Eq, PartialEq)]
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
    pub upkeep: u64,
    pub build_time: u64,
}

#[derive(Debug, Clone)]
pub struct ResourceGroup(u64, u64, u64, u64);

impl ResourceGroup {
    pub const fn new(lumber: u64, clay: u64, iron: u64, crop: u64) -> Self {
        Self(lumber, clay, iron, crop)
    }
}

pub type SmithyUpgrades = [u8; 10];

#[derive(Debug, Clone)]
pub struct Player {
    pub id: String,
    pub username: String,
    pub tribe: Tribe,
}
