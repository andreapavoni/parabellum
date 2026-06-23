//! Hero use-case inputs.
//!
//! These request types describe player intent for creating, reviving, and
//! updating heroes.

use parabellum_game::models::hero::HeroResourceFocus;
use uuid::Uuid;

/// Player request to load the current hero for a player.
#[derive(Debug, Clone, Copy)]
pub struct GetHeroByPlayerRequest {
    /// Player whose hero should be loaded.
    pub player_id: Uuid,
}

/// Player request to load the pending hero revival timestamp.
#[derive(Debug, Clone, Copy)]
pub struct GetPendingHeroRevivalRequest {
    /// Player whose pending hero revival should be loaded.
    pub player_id: Uuid,
}

/// Player request to create a village hero.
#[derive(Debug, Clone)]
pub struct CreateHeroRequest {
    /// Deterministic hero id supplied by the caller.
    pub hero_id: Uuid,
    /// Player expected to own the village.
    pub player_id: Uuid,
    /// Village where the hero should be created.
    pub village_id: u32,
}

/// Player request to revive a dead hero.
#[derive(Debug, Clone)]
pub struct ReviveHeroRequest {
    /// Hero to revive.
    pub hero_id: Uuid,
    /// Player expected to own the hero and village.
    pub player_id: Uuid,
    /// Village where revival should be queued.
    pub village_id: u32,
    /// Whether hero points should be reset on revival.
    pub reset: bool,
}

/// Player request to assign available hero attribute points.
#[derive(Debug, Clone)]
pub struct AssignHeroPointsRequest {
    /// Hero to update.
    pub hero_id: Uuid,
    /// Player expected to own the hero and village.
    pub player_id: Uuid,
    /// Village where the hero is expected to belong.
    pub village_id: u32,
    /// Strength points to assign.
    pub strength: u16,
    /// Offensive bonus points to assign.
    pub off_bonus: u16,
    /// Defensive bonus points to assign.
    pub def_bonus: u16,
    /// Regeneration points to assign.
    pub regeneration: u16,
    /// Resource production points to assign.
    pub resources: u16,
}

/// Player request to reset level-zero hero points.
#[derive(Debug, Clone)]
pub struct ResetHeroPointsRequest {
    /// Hero to update.
    pub hero_id: Uuid,
    /// Player expected to own the hero and village.
    pub player_id: Uuid,
    /// Village where the hero is expected to belong.
    pub village_id: u32,
}

/// Player request to change hero resource production focus.
#[derive(Debug, Clone)]
pub struct SetHeroResourceFocusRequest {
    /// Hero to update.
    pub hero_id: Uuid,
    /// Player expected to own the hero and village.
    pub player_id: Uuid,
    /// Village where the hero is expected to belong.
    pub village_id: u32,
    /// New resource focus.
    pub focus: HeroResourceFocus,
}
