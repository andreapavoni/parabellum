//! Projected village state model.

use chrono::{DateTime, Utc};
use parabellum_game::models::{
    smithy::SmithyUpgrades,
    trapper::TrapperState,
    village::{AcademyResearch, VillageBuilding, VillageProduction, VillageStocks},
};
use parabellum_types::{map::Position, tribe::Tribe};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Projected village state used by app reads and ES workflows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageModel {
    pub village_id: u32,
    pub player_id: Uuid,
    pub village_name: String,
    pub position: Position,
    pub tribe: Tribe,
    pub buildings: Vec<VillageBuilding>,
    pub production: VillageProduction,
    pub stocks: VillageStocks,
    pub population: u32,
    pub loyalty: u8,
    pub loyalty_updated_at: DateTime<Utc>,
    pub is_capital: bool,
    pub culture_points_production: u32,
    pub smithy_upgrades: SmithyUpgrades,
    pub academy_research: AcademyResearch,
    pub total_merchants: u8,
    pub busy_merchants: u8,
    pub trapper: TrapperState,
    pub updated_at: DateTime<Utc>,
    pub parent_village_id: Option<u32>,
}
