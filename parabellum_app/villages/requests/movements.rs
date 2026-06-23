//! Movement dispatch use-case inputs.
//!
//! These request types describe player intent for outbound movement. The
//! movement use case loads source/target context, asks the domain layer for
//! movement mechanics, and converts these inputs into aggregate command intent.

use parabellum_types::{
    army::TroopSet,
    battle::{AttackType, ScoutingTarget},
    buildings::BuildingName,
    map::Position,
    tribe::Tribe,
};
use uuid::Uuid;

/// Player request to send troops as reinforcement to another village.
#[derive(Debug, Clone)]
pub struct SendReinforcementRequest {
    /// Player expected to own the source village.
    pub player_id: Uuid,
    /// Village dispatching the army.
    pub source_village_id: u32,
    /// Destination village receiving reinforcement.
    pub target_village_id: u32,
    /// Units selected from the source village army.
    pub units: TroopSet,
    /// Optional hero to dispatch with the selected units.
    pub hero_id: Option<Uuid>,
}

/// Player request to send an attack or raid to another village.
#[derive(Debug, Clone)]
pub struct SendAttackRequest {
    /// Player expected to own the source village.
    pub player_id: Uuid,
    /// Village dispatching the attack.
    pub source_village_id: u32,
    /// Destination village being attacked.
    pub target_village_id: u32,
    /// Units selected from the source village army.
    pub units: TroopSet,
    /// Optional hero to dispatch with the selected units.
    pub hero_id: Option<Uuid>,
    /// Attack mode used by battle resolution.
    pub attack_type: AttackType,
    /// Optional catapult targets selected for the attack.
    pub catapult_targets: [Option<BuildingName>; 2],
}

/// Player request to scout another village.
#[derive(Debug, Clone)]
pub struct SendScoutRequest {
    /// Player expected to own the source village.
    pub player_id: Uuid,
    /// Village dispatching the scouts.
    pub source_village_id: u32,
    /// Destination village being scouted.
    pub target_village_id: u32,
    /// Scout units selected from the source village army.
    pub units: TroopSet,
    /// Scouting information target.
    pub target: ScoutingTarget,
    /// Scouting mode, usually normal attack or raid semantics.
    pub attack_type: AttackType,
}

/// Player request to found a new village with settlers.
#[derive(Debug, Clone)]
pub struct SendSettlersRequest {
    /// Player expected to own the source village.
    pub player_id: Uuid,
    /// Village dispatching settlers.
    pub source_village_id: u32,
    /// Empty map position where the new village should be founded.
    pub target_position: Position,
    /// Name assigned to the new village.
    pub village_name: String,
    /// Tribe for the founded village.
    pub tribe: Tribe,
}
