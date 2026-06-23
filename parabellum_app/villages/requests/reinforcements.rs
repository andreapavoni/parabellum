//! Reinforcement and trapped-troop control use-case inputs.
//!
//! These request types describe player intent for controlling stationed
//! reinforcements and trapped armies. Use cases load current app context and
//! translate them into command/workflow intent.

use parabellum_types::army::TroopSet;
use uuid::Uuid;

/// Player request to recall troops stationed as reinforcement elsewhere.
#[derive(Debug, Clone)]
pub struct RecallReinforcementsRequest {
    /// Player expected to own the home village and reinforcement army.
    pub player_id: Uuid,
    /// Home village that owns the stationed army.
    pub village_id: u32,
    /// Stationed army to recall from its current village.
    pub army_id: Uuid,
    /// Units selected to return home.
    pub units: TroopSet,
    /// Optional hero selected to return home.
    pub hero_id: Option<Uuid>,
}

/// Player request to release troops stationed in one of their villages.
#[derive(Debug, Clone)]
pub struct ReleaseReinforcementsRequest {
    /// Player expected to own the stationed village that is releasing the army.
    pub player_id: Uuid,
    /// Home village that owns the stationed army.
    pub village_id: u32,
    /// Stationed army to release from its current village.
    pub army_id: Uuid,
    /// Units selected to return home.
    pub units: TroopSet,
    /// Optional hero selected to return home.
    pub hero_id: Option<Uuid>,
}

/// Player request to release an enemy trapped army from one of their villages.
#[derive(Debug, Clone)]
pub struct ReleaseTrappedTroopsRequest {
    /// Player expected to own the village holding the trapped army.
    pub player_id: Uuid,
    /// Village currently holding the trapped army.
    pub village_id: u32,
    /// Trapped army to release.
    pub army_id: Uuid,
}

/// Player request to disband one of their armies trapped in another village.
#[derive(Debug, Clone)]
pub struct DisbandTrappedTroopsRequest {
    /// Player expected to own the trapped army's home village.
    pub player_id: Uuid,
    /// Home village that owns the trapped army.
    pub village_id: u32,
    /// Trapped army to disband.
    pub army_id: Uuid,
}
