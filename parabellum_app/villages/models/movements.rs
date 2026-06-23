//! Projected village movement models.

use chrono::{DateTime, Utc};
use parabellum_types::{common::ResourceGroup, map::Position, tribe::Tribe};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Projected movement category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementType {
    Attack,
    Raid,
    Scout,
    Reinforcement,
    Return,
    FoundVillage,
}

/// Direction of a projected movement from the viewing village perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementDirection {
    Incoming,
    Outgoing,
}

/// Projected movement row used by village activity reads.
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
    #[serde(default)]
    pub has_hero: bool,
    pub tribe: Option<Tribe>,
    pub bounty: Option<ResourceGroup>,
}

/// Movement rows split by viewing direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageTroopMovements {
    pub outgoing: Vec<VillageMovement>,
    pub incoming: Vec<VillageMovement>,
}
