use chrono::{DateTime, Utc};
use parabellum_game::models::{
    hero::Hero,
    smithy::SmithyUpgrades,
    village::{AcademyResearch, VillageBuilding, VillageProduction, VillageStocks},
};
use parabellum_types::battle::AttackType;
use parabellum_types::battle::ScoutingTarget;
use parabellum_types::buildings::BuildingName;
use parabellum_types::common::{ResourceGroup, ResourceQuantity};
use parabellum_types::reports::ReportPayload;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
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
    pub loyalty_updated_at: DateTime<Utc>,
    pub is_capital: bool,
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
    pub bounty: Option<ResourceGroup>,
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

impl fmt::Display for ScheduledActionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        };
        f.write_str(value)
    }
}

impl FromStr for ScheduledActionStatus {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "processing" => Ok(Self::Processing),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err("invalid scheduled action status"),
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingWorkflowKind {
    Add,
    Upgrade,
    Downgrade,
}

impl BuildingWorkflowKind {
    pub fn action_type(&self) -> ScheduledActionType {
        match self {
            Self::Add => ScheduledActionType::AddBuilding,
            Self::Upgrade => ScheduledActionType::UpgradeBuilding,
            Self::Downgrade => ScheduledActionType::DowngradeBuilding,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingWorkflow {
    pub kind: BuildingWorkflowKind,
    pub village_id: u32,
    pub player_id: Uuid,
    pub slot_id: u8,
    pub building_name: BuildingName,
    pub level: u8,
    pub speed: i8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingWorkflow {
    pub village_id: u32,
    pub player_id: Uuid,
    pub slot_id: u8,
    pub unit: parabellum_types::army::UnitName,
    pub time_per_unit: i32,
    pub quantity_remaining: i32,
    pub execute_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchWorkflowKind {
    Academy,
    Smithy,
}

impl ResearchWorkflowKind {
    pub fn action_type(&self) -> ScheduledActionType {
        match self {
            Self::Academy => ScheduledActionType::ResearchAcademy,
            Self::Smithy => ScheduledActionType::ResearchSmithy,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchWorkflow {
    pub kind: ResearchWorkflowKind,
    pub village_id: u32,
    pub player_id: Uuid,
    pub unit: parabellum_types::army::UnitName,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroRevivalWorkflow {
    pub village_id: u32,
    pub player_id: Uuid,
    pub hero: Hero,
    pub reset: bool,
    pub revive_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantArrivalWorkflow {
    pub village_id: u32,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub player_id: Uuid,
    pub resources: ResourceGroup,
    pub merchants_used: u8,
    pub arrives_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantReturnWorkflow {
    pub village_id: u32,
    pub source_village_id: u32,
    pub target_village_id: Option<u32>,
    pub player_id: Uuid,
    pub merchants_used: u8,
    pub returns_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmyReturnWorkflow {
    pub village_id: u32,
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub player_id: Uuid,
    pub army: Army,
    pub bounty: Option<ResourceGroup>,
    pub returns_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReinforcementArrivalWorkflow {
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub army: Army,
    pub arrives_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoutArrivalWorkflow {
    pub action_id: Uuid,
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub return_action_id: Uuid,
    pub village_id: u32,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub player_id: Uuid,
    pub army: Army,
    pub target: ScoutingTarget,
    pub attack_type: AttackType,
    pub arrives_at: DateTime<Utc>,
    pub returns_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettlersArrivalWorkflow {
    pub action_id: Uuid,
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub village_id: u32,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub target_position: Position,
    pub player_id: Uuid,
    pub village_name: String,
    pub tribe: Tribe,
    pub arrives_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttackArrivalWorkflow {
    pub action_id: Uuid,
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub return_action_id: Uuid,
    pub village_id: u32,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub player_id: Uuid,
    pub army: Army,
    pub attack_type: AttackType,
    pub catapult_targets: [Option<BuildingName>; 2],
    pub arrives_at: DateTime<Utc>,
    pub returns_at: DateTime<Utc>,
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
        workflow: ReinforcementArrivalWorkflow,
    },
    SettlersArrival {
        workflow: SettlersArrivalWorkflow,
    },
    AttackArrival {
        workflow: AttackArrivalWorkflow,
    },
    ArmyReturn {
        workflow: ArmyReturnWorkflow,
    },
    ScoutArrival {
        workflow: ScoutArrivalWorkflow,
    },
    MerchantsArrival {
        workflow: MerchantArrivalWorkflow,
    },
    MerchantsReturn {
        workflow: MerchantReturnWorkflow,
    },
    Building {
        workflow: BuildingWorkflow,
    },
    Training {
        workflow: TrainingWorkflow,
    },
    Research {
        workflow: ResearchWorkflow,
    },
    HeroRevival {
        workflow: HeroRevivalWorkflow,
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
            ScheduledActionPayload::Building { workflow } => workflow.kind.action_type(),
            ScheduledActionPayload::Training { .. } => ScheduledActionType::TrainUnit,
            ScheduledActionPayload::Research { workflow } => workflow.kind.action_type(),
            ScheduledActionPayload::HeroRevival { .. } => ScheduledActionType::HeroRevival,
        }
    }
}
