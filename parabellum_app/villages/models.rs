use chrono::{DateTime, Utc};
use parabellum_game::models::{
    hero::Hero,
    smithy::SmithyUpgrades,
    village::{AcademyResearch, VillageBuilding, VillageProduction, VillageStocks},
};
use parabellum_types::battle::AttackType;
use parabellum_types::battle::ScoutingTarget;
use parabellum_types::buildings::BuildingName;
use parabellum_types::common::ResourceQuantity;
use parabellum_types::reports::ReportPayload;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_game::models::army::Army;
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
    pub smithy_upgrades: SmithyUpgrades,
    pub academy_research: AcademyResearch,
    pub total_merchants: u8,
    pub busy_merchants: u8,
    pub updated_at: DateTime<Utc>,
    pub parent_village_id: Option<u32>,
    pub army: Option<Army>,
    pub reinforcements: Vec<Army>,
    pub deployed_armies: Vec<Army>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementType {
    Attack,
    Raid,
    Scout,
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
    pub units: parabellum_types::army::TroopSet,
    pub tribe: Option<Tribe>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageTroopMovements {
    pub outgoing: Vec<VillageMovement>,
    pub incoming: Vec<VillageMovement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketplaceOfferStatus {
    Open,
    Accepted,
    Canceled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceOfferModel {
    pub offer_id: Uuid,
    pub owner_player_id: Uuid,
    pub owner_village_id: u32,
    pub offer_resources: ResourceQuantity,
    pub seek_resources: ResourceQuantity,
    pub merchants_reserved: u8,
    pub status: MarketplaceOfferStatus,
    pub accepted_by_player_id: Option<Uuid>,
    pub accepted_by_village_id: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub canceled_at: Option<DateTime<Utc>>,
}

/// Domain snapshot used for marketplace offer command orchestration.
///
/// This is intentionally decoupled from projection-specific read model structs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceOfferSnapshot {
    pub offer_id: Uuid,
    pub owner_player_id: Uuid,
    pub owner_village_id: u32,
    pub offer_resources: ResourceQuantity,
    pub seek_resources: ResourceQuantity,
    pub merchants_reserved: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportModel {
    pub id: Uuid,
    pub report_type: String,
    pub payload: ReportPayload,
    pub actor_player_id: Uuid,
    pub actor_village_id: Option<u32>,
    pub target_player_id: Option<Uuid>,
    pub target_village_id: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
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
    SettlersArrival,
    AttackArrival,
    ArmyReturn,
    ScoutArrival,
    MerchantsArrival,
    MerchantsReturn,
    AddBuilding,
    UpgradeBuilding,
    DowngradeBuilding,
    TrainUnit,
    ResearchAcademy,
    ResearchSmithy,
    HeroRevival,
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
        army: Army,
        arrives_at: DateTime<Utc>,
    },
    SettlersArrival {
        action_id: Uuid,
        movement_id: Uuid,
        army_id: Uuid,
        village_id: u32,
        source_village_id: u32,
        target_village_id: u32,
        target_position: Position,
        player_id: Uuid,
        village_name: String,
        tribe: Tribe,
        arrives_at: DateTime<Utc>,
    },
    AttackArrival {
        action_id: Uuid,
        movement_id: Uuid,
        army_id: Uuid,
        return_action_id: Uuid,
        village_id: u32,
        source_village_id: u32,
        target_village_id: u32,
        player_id: Uuid,
        army: Army,
        attack_type: AttackType,
        catapult_targets: [BuildingName; 2],
        arrives_at: DateTime<Utc>,
        returns_at: DateTime<Utc>,
    },
    ArmyReturn {
        action_id: Uuid,
        movement_id: Uuid,
        army_id: Uuid,
        village_id: u32,
        source_village_id: u32,
        target_village_id: u32,
        player_id: Uuid,
        army: Army,
        bounty: Option<parabellum_types::common::ResourceGroup>,
        returns_at: DateTime<Utc>,
    },
    ScoutArrival {
        action_id: Uuid,
        movement_id: Uuid,
        army_id: Uuid,
        return_action_id: Uuid,
        village_id: u32,
        source_village_id: u32,
        target_village_id: u32,
        player_id: Uuid,
        army: Army,
        target: ScoutingTarget,
        attack_type: AttackType,
        arrives_at: DateTime<Utc>,
        returns_at: DateTime<Utc>,
    },
    MerchantsArrival {
        action_id: Uuid,
        village_id: u32,
        source_village_id: u32,
        target_village_id: u32,
        player_id: Uuid,
        resources: parabellum_types::common::ResourceGroup,
        merchants_used: u8,
        arrives_at: DateTime<Utc>,
    },
    MerchantsReturn {
        action_id: Uuid,
        village_id: u32,
        source_village_id: u32,
        player_id: Uuid,
        merchants_used: u8,
        returns_at: DateTime<Utc>,
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
    TrainUnit {
        action_id: Uuid,
        village_id: u32,
        player_id: Uuid,
        slot_id: u8,
        unit: parabellum_types::army::UnitName,
        time_per_unit: i32,
        quantity_remaining: i32,
        execute_at: DateTime<Utc>,
    },
    ResearchAcademy {
        action_id: Uuid,
        village_id: u32,
        player_id: Uuid,
        unit: parabellum_types::army::UnitName,
    },
    ResearchSmithy {
        action_id: Uuid,
        village_id: u32,
        player_id: Uuid,
        unit: parabellum_types::army::UnitName,
    },
    HeroRevival {
        action_id: Uuid,
        village_id: u32,
        player_id: Uuid,
        hero: Hero,
        reset: bool,
        revive_at: DateTime<Utc>,
    },
}

impl ScheduledActionPayload {
    pub fn action_type(&self) -> ScheduledActionType {
        match self {
            ScheduledActionPayload::ReinforcementArrival { .. } => {
                ScheduledActionType::ReinforcementArrival
            }
            ScheduledActionPayload::SettlersArrival { .. } => ScheduledActionType::SettlersArrival,
            ScheduledActionPayload::AttackArrival { .. } => ScheduledActionType::AttackArrival,
            ScheduledActionPayload::ArmyReturn { .. } => ScheduledActionType::ArmyReturn,
            ScheduledActionPayload::ScoutArrival { .. } => ScheduledActionType::ScoutArrival,
            ScheduledActionPayload::MerchantsArrival { .. } => {
                ScheduledActionType::MerchantsArrival
            }
            ScheduledActionPayload::MerchantsReturn { .. } => ScheduledActionType::MerchantsReturn,
            ScheduledActionPayload::AddBuilding { .. } => ScheduledActionType::AddBuilding,
            ScheduledActionPayload::UpgradeBuilding { .. } => ScheduledActionType::UpgradeBuilding,
            ScheduledActionPayload::DowngradeBuilding { .. } => {
                ScheduledActionType::DowngradeBuilding
            }
            ScheduledActionPayload::TrainUnit { .. } => ScheduledActionType::TrainUnit,
            ScheduledActionPayload::ResearchAcademy { .. } => ScheduledActionType::ResearchAcademy,
            ScheduledActionPayload::ResearchSmithy { .. } => ScheduledActionType::ResearchSmithy,
            ScheduledActionPayload::HeroRevival { .. } => ScheduledActionType::HeroRevival,
        }
    }
}
