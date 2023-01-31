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
pub struct ResourceGroup {
    pub lumber: u64,
    pub clay: u64,
    pub iron: u64,
    pub crop: u64,
}

pub type SmithyUpgrades = [u8; 10];

#[derive(Debug, Clone)]
pub struct Player {
    pub id: String,
    pub user_id: String,
    pub tribe: Tribe,
}
