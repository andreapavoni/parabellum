//! Village activity read models.
//!
//! Activity views summarize scheduled village work and troop movement state for
//! application/UI reads. They deliberately stay separate from command payloads
//! and persistence projection rows.

use chrono::{DateTime, Utc};
use parabellum_types::{
    army::{TroopSet, UnitName},
    buildings::BuildingName,
    common::ResourceGroup,
    map::Position,
};
use uuid::Uuid;

use crate::villages::models::{BuildingWorkflowKind, ScheduledActionStatus};

/// A pending building construction, upgrade, downgrade, or cancellation target.
#[derive(Debug, Clone)]
pub struct BuildingQueueItem {
    pub job_id: Uuid,
    pub kind: BuildingWorkflowKind,
    pub slot_id: u8,
    pub building_name: BuildingName,
    pub target_level: u8,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

/// A pending unit-training queue item.
#[derive(Debug, Clone)]
pub struct TrainingQueueItem {
    pub job_id: Uuid,
    pub slot_id: u8,
    pub unit: UnitName,
    pub quantity: i32,
    pub time_per_unit: i32,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

/// A pending academy research queue item.
#[derive(Debug, Clone)]
pub struct AcademyQueueItem {
    pub job_id: Uuid,
    pub unit: UnitName,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

/// A pending smithy upgrade queue item.
#[derive(Debug, Clone)]
pub struct SmithyQueueItem {
    pub job_id: Uuid,
    pub unit: UnitName,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

/// A pending trap-building queue item.
#[derive(Debug, Clone)]
pub struct TrapQueueItem {
    pub job_id: Uuid,
    pub quantity: i32,
    pub time_per_trap: i32,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

/// Queue summary for all scheduled village work shown in village activity UI.
#[derive(Debug, Clone, Default)]
pub struct VillageQueues {
    pub building: Vec<BuildingQueueItem>,
    pub training: Vec<TrainingQueueItem>,
    pub academy: Vec<AcademyQueueItem>,
    pub smithy: Vec<SmithyQueueItem>,
    pub traps: Vec<TrapQueueItem>,
}

/// App-facing troop movement category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TroopMovementType {
    Attack,
    Raid,
    Scout,
    Reinforcement,
    Return,
    FoundVillage,
}

/// Whether a troop movement is arriving at or leaving the selected village.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TroopMovementDirection {
    Incoming,
    Outgoing,
}

/// App-facing troop movement summary.
#[derive(Debug, Clone, PartialEq)]
pub struct TroopMovement {
    pub job_id: Uuid,
    pub movement_type: TroopMovementType,
    pub direction: TroopMovementDirection,
    pub origin_village_id: u32,
    pub origin_village_name: Option<String>,
    pub origin_player_id: Uuid,
    pub origin_position: Position,
    pub target_village_id: u32,
    pub target_village_name: Option<String>,
    pub target_player_id: Uuid,
    pub target_position: Position,
    pub arrives_at: DateTime<Utc>,
    pub time_seconds: u32,
    pub units: TroopSet,
    pub has_hero: bool,
    pub tribe: parabellum_types::tribe::Tribe,
    pub bounty: Option<ResourceGroup>,
}

/// Incoming and outgoing troop movement summary for a village.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VillageTroopMovements {
    pub outgoing: Vec<TroopMovement>,
    pub incoming: Vec<TroopMovement>,
}
