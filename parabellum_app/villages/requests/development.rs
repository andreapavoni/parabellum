//! Village development use-case inputs.
//!
//! These request types describe player intent for unit training and research.

use parabellum_types::{army::UnitName, buildings::BuildingName};
use uuid::Uuid;

/// Player request to queue unit training in a village building.
#[derive(Debug, Clone)]
pub struct TrainUnitsRequest {
    /// Player expected to own the village.
    pub player_id: Uuid,
    /// Village where training should be queued.
    pub village_id: u32,
    /// Tribe-local unit index to train.
    pub unit_idx: u8,
    /// Building used for training.
    pub building_name: BuildingName,
    /// Number of units to train.
    pub quantity: i32,
}

/// Player request to queue academy research for a unit.
#[derive(Debug, Clone)]
pub struct ResearchAcademyRequest {
    /// Player expected to own the village.
    pub player_id: Uuid,
    /// Village where academy research should be queued.
    pub village_id: u32,
    /// Unit to research.
    pub unit: UnitName,
}

/// Player request to queue smithy research for a unit.
#[derive(Debug, Clone)]
pub struct ResearchSmithyRequest {
    /// Player expected to own the village.
    pub player_id: Uuid,
    /// Village where smithy research should be queued.
    pub village_id: u32,
    /// Unit to upgrade.
    pub unit: UnitName,
}
