use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::game::models::{ResourceGroup, army::UnitName, buildings::BuildingName};

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
    pub time_per_unit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidTask {
    pub army_id: Uuid,
    pub village_id: i32,
    pub player_id: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinforcementTask {
    pub parmy_id: Uuid,
    pub pvillage_id: i32,
    pub pplayer_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantGoingTask {
    pub resources: ResourceGroup,
    pub village_id: i32,
    pub player_id: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantReturnTask {
    pub village_id: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainBarracks {
    pub slot_id: u8,
    pub unit: UnitName,
    pub quantity: i32,
    pub time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainGreatBarracksTask {
    pub slot_id: u8,
    pub unit: UnitName,
    pub quantity: i32,
    pub time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainStableTask {
    pub slot_id: u8,
    pub unit: UnitName,
    pub quantity: i32,
    pub time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainGreatStableTask {
    pub slot_id: u8,
    pub unit: UnitName,
    pub quantity: i32,
    pub time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainWorkshopTask {
    pub slot_id: u8,
    pub unit: UnitName,
    pub quantity: i32,
    pub time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainGreatWorkshopTask {
    pub slot_id: u8,
    pub unit: UnitName,
    pub quantity: i32,
    pub time_per_unit_secs: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainExpansionTask {
    pub slot_id: u8,
    pub unit: UnitName,
    pub quantity: i32,
    pub time_per_unit_secs: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBuildingTask {
    pub slot_id: u8,
    pub name: BuildingName,
    pub village_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingUpgradeTask {
    pub slot_id: u8,
    pub building_name: BuildingName,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingDowngradeTask {
    pub slot_id: u8,
    pub building_name: BuildingName,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchAcademyTask {
    pub unit: UnitName,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSmithyTask {
    pub unit: UnitName,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CelebrationTownHallTask {
    pub big: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CelebrationBreweryTask {}
