use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::game::models::{army::UnitName, buildings::BuildingName, ResourceGroup};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmyReturnTask {
    pub army_id: Uuid,
    pub resources: ResourceGroup,
    pub destination_village_id: i32,
    pub destination_player_id: Uuid,
    pub from_village_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackTask {
    pub army_id: Uuid,
    pub attacker_village_id: i32,
    pub attacker_player_id: Uuid,
    pub target_village_id: i32,
    pub target_player_id: Uuid,
    pub catapult_targets: [BuildingName; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainUnitsTask {
    pub slot_id: u8,
    pub unit: UnitName,
    pub quantity: i32,
    pub time_per_unit_secs: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidTask {
    army_id: Uuid,
    village_id: i32,
    player_id: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinforcementTask {
    army_id: Uuid,
    village_id: i32,
    player_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantGoingTask {
    resources: ResourceGroup,
    village_id: i32,
    player_id: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantReturnTask {
    village_id: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainBarracks {
    slot_id: u8,
    unit: UnitName,
    quantity: i32,
    time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainGreatBarracksTask {
    slot_id: u8,
    unit: UnitName,
    quantity: i32,
    time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainStableTask {
    slot_id: u8,
    unit: UnitName,
    quantity: i32,
    time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainGreatStableTask {
    slot_id: u8,
    unit: UnitName,
    quantity: i32,
    time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainWorkshopTask {
    slot_id: u8,
    unit: UnitName,
    quantity: i32,
    time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainGreatWorkshopTask {
    slot_id: u8,
    unit: UnitName,
    quantity: i32,
    time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainExpansionTask {
    slot_id: u8,
    unit: UnitName,
    quantity: i32,
    time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingUpgradeTask {
    slot_id: u8,
    building_name: BuildingName,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingDowngradeTask {
    slot_id: u8,
    building_name: BuildingName,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchAcademyTask {
    unit: UnitName,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSmithyTask {
    unit: UnitName,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CelebrationTownHallTask {
    big: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CelebrationBreweryTask {}
