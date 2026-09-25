//! Application-owned building previews and the context used by feature enrichers.
use crate::villages::{models::VillageModel, read_models::VillageQueues};
use chrono::{DateTime, Utc};
use parabellum_game::models::village::VillageBuilding;
use parabellum_types::{
    buildings::{BuildingName, BuildingRequirement},
    common::ResourceGroup,
};

/// Common building query result. Feature-specific reads are deliberately deferred.
#[derive(Debug, Clone)]
pub struct BuildingOverview {
    pub server_time: DateTime<Utc>,
    pub village: VillageModel,
    pub queues: VillageQueues,
    pub queue_full: bool,
    pub slot: BuildingSlotOverview,
}

/// Occupied and empty slots have distinct preview data.
#[derive(Debug, Clone)]
pub enum BuildingSlotOverview {
    Occupied {
        building: VillageBuilding,
        upgrade: BuildingUpgradePreview,
    },
    Empty(EmptyBuildingSlot),
}

/// Cost and effect of the next level, including already committed upgrades.
#[derive(Debug, Clone)]
pub struct BuildingUpgradePreview {
    pub building_name: BuildingName,
    pub current_level: u8,
    pub next_level: u8,
    pub current_upkeep: u32,
    pub next_upkeep: u32,
    pub time_secs: u32,
    pub at_max_level: bool,
    pub current_value: Option<u32>,
    pub next_value: Option<u32>,
    pub cost: ResourceGroup,
}

/// Construction choices and any committed construction on an empty slot.
#[derive(Debug, Clone)]
pub struct EmptyBuildingSlot {
    pub buildable_buildings: Vec<BuildingOption>,
    pub locked_buildings: Vec<BuildingOption>,
    pub has_queue_for_slot: bool,
    pub queued_building_name: Option<BuildingName>,
    pub queued_target_level: Option<u8>,
    pub queued_next_level: Option<u8>,
    pub queued_can_upgrade: Option<bool>,
    pub queued_upgrade_preview: Option<BuildingUpgradePreview>,
}

/// Structural availability is independent of affordability, exposed via costs.
#[derive(Debug, Clone)]
pub struct BuildingOption {
    pub building_name: BuildingName,
    pub cost: ResourceGroup,
    pub upkeep: u32,
    pub time_secs: u32,
    pub missing_requirements: Vec<BuildingRequirement>,
}
