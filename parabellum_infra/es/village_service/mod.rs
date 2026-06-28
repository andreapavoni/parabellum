//! Village ES orchestration service.
//!
//! This module is intentionally split by concern:
//! - `mod.rs`: facade type, exported context structs, and module index
//! - `commands.rs`: direct CQRS command dispatch helpers
//! - `marketplace_commands.rs`: marketplace command orchestration
//! - `economy.rs`: pre-command economy materialization helpers
//! - `workflow_append.rs`: cross-stream workflow append mechanics
//! - `queries/`: read/query helpers consumed by adapters/web layer
//! - `scheduler.rs`: deterministic fact-driven scheduled workflow progression
//!
//! Public API remains centered on `VillageEsService`.

use parabellum_game::models::army::Army;
use parabellum_types::common::ResourceGroup;
use sqlx::PgPool;
use uuid::Uuid;

mod commands;
mod economy;
mod marketplace_commands;
mod queries;
mod scheduler;
mod workflow_append;

#[derive(Debug, Clone)]
/// ES orchestration facade for village command, scheduler, and read helper flows.
pub struct VillageEsService {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct ReinforcementContext {
    /// Village where the reinforcement is currently stationed.
    pub stationed_village_id: u32,
    /// Home/origin village of the reinforcement army.
    pub home_village_id: u32,
    /// Full army state for recall/release command construction.
    pub army: Army,
}

#[derive(Debug, Clone)]
pub struct TrappedArmyContext {
    pub trapped_village_id: u32,
    pub home_village_id: u32,
    pub army: Army,
}

pub struct CancelTroopMovementContext {
    pub movement_id: Uuid,
    pub arrival_action_id: Uuid,
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub army: Army,
    pub sent_at: chrono::DateTime<chrono::Utc>,
    pub arrives_at: chrono::DateTime<chrono::Utc>,
}

pub struct CancelBuildingConstructionContext {
    pub action_ids: Vec<Uuid>,
    pub player_id: Uuid,
    pub village_id: u32,
    pub execute_at: chrono::DateTime<chrono::Utc>,
    pub refund: ResourceGroup,
}

impl VillageEsService {
    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
