//! Identity and registration request models.
//!
//! These inputs are transport-independent application requests. They are used
//! by HTTP handlers, seed tooling, and identity infrastructure adapters.

use parabellum_game::models::{map::MapQuadrant, village::VillageBuilding};
use parabellum_types::{common::ResourceGroup, tribe::Tribe};
use uuid::Uuid;

/// Optional deterministic setup for a player's initial village.
///
/// Normal registration can omit this and let runtime defaults apply. Seed and
/// test flows use it to create predictable village state while still passing
/// through the registration boundary.
#[derive(Debug, Clone)]
pub struct InitialVillageSetup {
    pub village_name: Option<String>,
    pub resource_fields_target_level: u8,
    pub buildings: Vec<VillageBuilding>,
    pub resources: Option<ResourceGroup>,
    pub speed: Option<i8>,
}

/// Registers a player, user credentials, and initial village.
///
/// This request intentionally describes the whole registration intent. The
/// current infrastructure implementation still performs the transactional work;
/// a later registration slice should move orchestration into app use cases and
/// keep infra behind smaller ports.
#[derive(Debug, Clone)]
pub struct RegisterPlayerRequest {
    pub player_id: Uuid,
    pub username: String,
    pub email: String,
    pub password: String,
    pub tribe: Tribe,
    pub quadrant: MapQuadrant,
    pub initial_village: Option<InitialVillageSetup>,
}
