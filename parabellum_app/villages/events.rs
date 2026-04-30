//! Village aggregate domain events for CQRS/ES flows.
//!
//! Event families:
//! - immediate actions (e.g. `VillageFounded`, `ReinforcementSent`)
//! - scheduled actions (`*Scheduled`)
//! - deterministic completions (`*Completed`, `Building*`)
//! - utility updates (`VillageResourcesSet`)
use std::fmt;

use chrono::{DateTime, Utc};
use mini_cqrs_es::EventPayload;
use parabellum_game::models::village::VillageBuilding;
use parabellum_types::army::{TroopSet, UnitName};
use parabellum_types::buildings::BuildingName;
use parabellum_types::common::ResourceGroup;
use parabellum_types::map::Position;
use parabellum_types::tribe::Tribe;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VillageEvent {
    VillageFounded {
        village_id: u32,
        village_name: String,
        position: Position,
        tribe: Tribe,
        player_id: Uuid,
        stationed_units: TroopSet,
        buildings: Vec<VillageBuilding>,
    },
    VillageConquered {
        player_id: Uuid,
    },
    /// Emitted when resources are explicitly set through `SetVillageResources`.
    ///
    /// Projectors should refresh resource-dependent read models from source.
    VillageResourcesSet {
        player_id: Uuid,
        village_id: u32,
        resources: ResourceGroup,
    },
    VillageArmyDetached {
        army_id: Uuid,
        units: TroopSet,
        hero_id: Option<Uuid>,
    },
    ReinforcementSent {
        movement_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        units: TroopSet,
        hero_id: Option<Uuid>,
        arrives_at: DateTime<Utc>,
    },
    ReinforcementArrived {
        movement_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        units: TroopSet,
        hero_id: Option<Uuid>,
        arrives_at: DateTime<Utc>,
    },
    BuildingConstructionScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
        execute_at: DateTime<Utc>,
    },
    BuildingUpgradeScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
        execute_at: DateTime<Utc>,
    },
    BuildingDowngradeScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
        execute_at: DateTime<Utc>,
    },
    BuildingAdded {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    },
    BuildingUpgraded {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    },
    BuildingDowngraded {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    },
    UnitTrainingScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        unit: UnitName,
        time_per_unit: i32,
        quantity_remaining: i32,
        execute_at: DateTime<Utc>,
    },
    UnitTrained {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        unit: UnitName,
        quantity_trained: u32,
    },
    AcademyResearchScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        unit: UnitName,
        execute_at: DateTime<Utc>,
    },
    AcademyResearchCompleted {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        unit: UnitName,
    },
    SmithyResearchScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        unit: UnitName,
        execute_at: DateTime<Utc>,
    },
    SmithyResearchCompleted {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        unit: UnitName,
    },
}

impl fmt::Display for VillageEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            VillageEvent::VillageFounded { .. } => "VillageFounded",
            VillageEvent::VillageConquered { .. } => "VillageConquered",
            VillageEvent::VillageResourcesSet { .. } => "VillageResourcesSet",
            VillageEvent::VillageArmyDetached { .. } => "VillageArmyDetached",
            VillageEvent::ReinforcementSent { .. } => "ReinforcementSent",
            VillageEvent::ReinforcementArrived { .. } => "ReinforcementArrived",
            VillageEvent::BuildingConstructionScheduled { .. } => "BuildingConstructionScheduled",
            VillageEvent::BuildingUpgradeScheduled { .. } => "BuildingUpgradeScheduled",
            VillageEvent::BuildingDowngradeScheduled { .. } => "BuildingDowngradeScheduled",
            VillageEvent::BuildingAdded { .. } => "BuildingAdded",
            VillageEvent::BuildingUpgraded { .. } => "BuildingUpgraded",
            VillageEvent::BuildingDowngraded { .. } => "BuildingDowngraded",
            VillageEvent::UnitTrainingScheduled { .. } => "UnitTrainingScheduled",
            VillageEvent::UnitTrained { .. } => "UnitTrained",
            VillageEvent::AcademyResearchScheduled { .. } => "AcademyResearchScheduled",
            VillageEvent::AcademyResearchCompleted { .. } => "AcademyResearchCompleted",
            VillageEvent::SmithyResearchScheduled { .. } => "SmithyResearchScheduled",
            VillageEvent::SmithyResearchCompleted { .. } => "SmithyResearchCompleted",
        };
        f.write_str(name)
    }
}

impl EventPayload for VillageEvent {}
