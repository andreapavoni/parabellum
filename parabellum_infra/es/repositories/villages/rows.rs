//! Typed rows and database enum adapters for village projections.

use parabellum_app::villages::models::VillageModel;
use parabellum_game::models::smithy::SmithyUpgrades;
use parabellum_game::models::trapper::TrapperState;
use parabellum_types::{errors::ApplicationError, tribe::Tribe};
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub(super) struct DbVillageModelRow {
    pub village_id: i32,
    pub player_id: Uuid,
    pub village_name: String,
    pub position: Json<parabellum_types::map::Position>,
    pub tribe: DbTribe,
    pub buildings: Json<Vec<parabellum_game::models::village::VillageBuilding>>,
    pub production: Json<parabellum_game::models::village::VillageProduction>,
    pub stocks: Json<parabellum_game::models::village::VillageStocks>,
    pub population: i32,
    pub loyalty: i16,
    pub is_capital: bool,
    pub culture_points_production: i32,
    pub smithy_upgrades: Json<SmithyUpgrades>,
    pub academy_research: Json<parabellum_game::models::village::AcademyResearch>,
    pub parent_village_id: Option<i32>,
    pub total_merchants: i16,
    pub busy_merchants: i16,
    pub trapper_active_traps: i32,
    pub trapper_broken_traps: i32,
    pub trapper_queued_traps: i32,
    pub loyalty_updated_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<DbVillageModelRow> for VillageModel {
    type Error = ApplicationError;

    fn try_from(value: DbVillageModelRow) -> Result<Self, Self::Error> {
        let buildings = value
            .buildings
            .0
            .into_iter()
            .filter(|building| (1..=18).contains(&building.slot_id) || building.building.level > 0)
            .collect();

        Ok(VillageModel {
            village_id: value.village_id as u32,
            player_id: value.player_id,
            village_name: value.village_name,
            position: value.position.0,
            tribe: value.tribe.into(),
            buildings,
            production: value.production.0,
            stocks: value.stocks.0,
            population: value.population as u32,
            loyalty: value.loyalty as u8,
            is_capital: value.is_capital,
            culture_points_production: value.culture_points_production as u32,
            smithy_upgrades: value.smithy_upgrades.0,
            academy_research: value.academy_research.0,
            total_merchants: value.total_merchants as u8,
            busy_merchants: value.busy_merchants as u8,
            trapper: TrapperState {
                active_traps: value.trapper_active_traps.max(0) as u32,
                broken_traps: value.trapper_broken_traps.max(0) as u32,
                queued_traps: value.trapper_queued_traps.max(0) as u32,
            },
            loyalty_updated_at: value.loyalty_updated_at,
            updated_at: value.updated_at,
            parent_village_id: value.parent_village_id.map(|v| v as u32),
        })
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "tribe", rename_all = "PascalCase")]
pub(super) enum DbTribe {
    Roman,
    Gaul,
    Teuton,
    Natar,
    Nature,
}

impl From<DbTribe> for Tribe {
    fn from(value: DbTribe) -> Self {
        match value {
            DbTribe::Roman => Self::Roman,
            DbTribe::Gaul => Self::Gaul,
            DbTribe::Teuton => Self::Teuton,
            DbTribe::Natar => Self::Natar,
            DbTribe::Nature => Self::Nature,
        }
    }
}

impl From<Tribe> for DbTribe {
    fn from(value: Tribe) -> Self {
        match value {
            Tribe::Roman => Self::Roman,
            Tribe::Gaul => Self::Gaul,
            Tribe::Teuton => Self::Teuton,
            Tribe::Natar => Self::Natar,
            Tribe::Nature => Self::Nature,
        }
    }
}
