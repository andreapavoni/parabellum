use chrono::{DateTime, Utc};
use parabellum_game::models::village::{VillageBuilding, VillageProduction, VillageStocks};
use parabellum_types::buildings::BuildingName;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_types::army::TroopSet;
use parabellum_types::map::Position;
use parabellum_types::tribe::Tribe;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageModel {
    pub village_id: u32,
    pub player_id: Uuid,
    pub village_name: String,
    pub position: Position,
    pub tribe: Tribe,
    pub buildings: Vec<VillageBuilding>,
    pub production: VillageProduction,
    pub stocks: VillageStocks,
    pub population: u32,
    pub loyalty: u8,
    pub is_capital: bool,
    pub culture_points: u32,
    pub culture_points_production: u32,
    pub total_merchants: u8,
    pub busy_merchants: u8,
    pub parent_village_id: Option<u32>,
    pub stationed_army: TroopSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementType {
    Attack,
    Raid,
    Reinforcement,
    Return,
    FoundVillage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageMovement {
    pub movement_id: Uuid,
    pub movement_type: MovementType,
    pub direction: MovementDirection,
    pub origin_village_id: u32,
    pub origin_village_name: Option<String>,
    pub origin_player_id: Uuid,
    pub origin_position: Option<Position>,
    pub target_village_id: u32,
    pub target_village_name: Option<String>,
    pub target_player_id: Option<Uuid>,
    pub target_position: Option<Position>,
    pub arrives_at: DateTime<Utc>,
    pub time_seconds: Option<u32>,
    pub units: TroopSet,
    pub tribe: Option<Tribe>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageTroopMovements {
    pub outgoing: Vec<VillageMovement>,
    pub incoming: Vec<VillageMovement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduledActionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduledActionType {
    ReinforcementArrival,
    AddBuilding,
    UpgradeBuilding,
    DowngradeBuilding,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledAction {
    pub id: Uuid,
    pub action_type: ScheduledActionType,
    pub execute_at: DateTime<Utc>,
    pub payload: serde_json::Value,
    pub status: ScheduledActionStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ScheduledActionPayload {
    ReinforcementArrival {
        movement_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        units: TroopSet,
        hero_id: Option<Uuid>,
        arrives_at: DateTime<Utc>,
    },
    AddBuilding {
        village_id: u32,
        player_id: Uuid,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    },
    UpgradeBuilding {
        village_id: u32,
        player_id: Uuid,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    },
    DowngradeBuilding {
        village_id: u32,
        player_id: Uuid,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    },
}

impl ScheduledActionPayload {
    pub fn action_type(&self) -> ScheduledActionType {
        match self {
            ScheduledActionPayload::ReinforcementArrival { .. } => {
                ScheduledActionType::ReinforcementArrival
            }
            ScheduledActionPayload::AddBuilding { .. } => ScheduledActionType::AddBuilding,
            ScheduledActionPayload::UpgradeBuilding { .. } => ScheduledActionType::UpgradeBuilding,
            ScheduledActionPayload::DowngradeBuilding { .. } => {
                ScheduledActionType::DowngradeBuilding
            }
        }
    }
}
