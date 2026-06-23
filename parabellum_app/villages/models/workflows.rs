//! Scheduled workflow payload models.

use chrono::{DateTime, Utc};
use parabellum_game::models::{army::Army, hero::Hero};
use parabellum_types::{
    battle::{AttackType, ScoutingTarget},
    buildings::BuildingName,
    common::ResourceGroup,
    map::Position,
    tribe::Tribe,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::scheduled_actions::ScheduledActionType;

/// Building workflow variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingWorkflowKind {
    Add,
    Upgrade,
    Downgrade,
}

impl BuildingWorkflowKind {
    /// Returns the scheduled action type represented by this building workflow.
    pub fn action_type(&self) -> ScheduledActionType {
        match self {
            Self::Add => ScheduledActionType::AddBuilding,
            Self::Upgrade => ScheduledActionType::UpgradeBuilding,
            Self::Downgrade => ScheduledActionType::DowngradeBuilding,
        }
    }
}

/// Workflow payload for building construction lifecycle jobs.
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

/// Workflow payload for unit-training lifecycle jobs.
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

/// Workflow payload for trap-building lifecycle jobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrapBuildWorkflow {
    pub village_id: u32,
    pub player_id: Uuid,
    pub quantity_remaining: i32,
    pub time_per_trap: i32,
    pub execute_at: DateTime<Utc>,
}

/// Workflow payload for returning troops released from traps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrappedTroopReturn {
    pub action_id: Uuid,
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub home_village_id: u32,
    pub trapped_village_id: u32,
    pub army: Army,
    pub returns_at: DateTime<Utc>,
}

/// Research workflow variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchWorkflowKind {
    Academy,
    Smithy,
}

impl ResearchWorkflowKind {
    /// Returns the scheduled action type represented by this research workflow.
    pub fn action_type(&self) -> ScheduledActionType {
        match self {
            Self::Academy => ScheduledActionType::ResearchAcademy,
            Self::Smithy => ScheduledActionType::ResearchSmithy,
        }
    }
}

/// Workflow payload for research lifecycle jobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchWorkflow {
    pub kind: ResearchWorkflowKind,
    pub village_id: u32,
    pub player_id: Uuid,
    pub unit: parabellum_types::army::UnitName,
}

/// Workflow payload for hero revival jobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroRevivalWorkflow {
    pub village_id: u32,
    pub player_id: Uuid,
    pub hero: Hero,
    pub reset: bool,
    pub revive_at: DateTime<Utc>,
}

/// Workflow payload for merchant arrival jobs.
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

/// Workflow payload for merchant return jobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantReturnWorkflow {
    pub village_id: u32,
    pub source_village_id: u32,
    pub target_village_id: Option<u32>,
    pub player_id: Uuid,
    pub merchants_used: u8,
    pub returns_at: DateTime<Utc>,
}

/// Workflow payload for army return jobs.
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

/// Workflow payload for reinforcement arrival jobs.
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

/// Workflow payload for scout arrival jobs.
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

/// Workflow payload for settlers arrival jobs.
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

/// Workflow payload for attack or raid arrival jobs.
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
