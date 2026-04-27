use uuid::Uuid;

use parabellum_game::models::village::Village;
use parabellum_types::errors::{ApplicationError, DbError};

use crate::{models as db_models, toasty_time::jiff_to_chrono_utc};

#[derive(Debug, Clone, toasty::Model)]
#[table = "villages"]
pub struct VillageDbRow {
    #[key]
    pub id: i32,

    #[index]
    pub player_id: Uuid,

    pub name: String,

    #[serialize(json)]
    pub position: serde_json::Value,

    #[serialize(json)]
    pub buildings: serde_json::Value,

    #[serialize(json)]
    pub production: serde_json::Value,

    #[serialize(json)]
    pub stocks: serde_json::Value,

    #[serialize(json)]
    pub smithy_upgrades: serde_json::Value,

    #[serialize(json)]
    pub academy_research: serde_json::Value,

    pub population: i32,
    pub loyalty: i16,
    pub is_capital: bool,
    pub culture_points: i32,
    pub culture_points_production: i32,
    pub created_at: jiff::Timestamp,
    pub updated_at: jiff::Timestamp,
    pub parent_village_id: Option<i32>,
}

impl TryFrom<VillageDbRow> for db_models::Village {
    type Error = ApplicationError;

    fn try_from(village: VillageDbRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: village.id,
            player_id: village.player_id,
            name: village.name,
            position: village.position,
            buildings: village.buildings,
            production: village.production,
            stocks: village.stocks,
            smithy_upgrades: village.smithy_upgrades,
            academy_research: village.academy_research,
            population: village.population,
            loyalty: village.loyalty,
            is_capital: village.is_capital,
            culture_points: village.culture_points,
            culture_points_production: village.culture_points_production,
            created_at: jiff_to_chrono_utc(village.created_at)?,
            updated_at: jiff_to_chrono_utc(village.updated_at)?,
            parent_village_id: village.parent_village_id,
        })
    }
}

impl TryFrom<&Village> for VillageDbRow {
    type Error = ApplicationError;

    fn try_from(village: &Village) -> Result<Self, Self::Error> {
        Ok(Self {
            id: village.id as i32,
            player_id: village.player_id,
            name: village.name.clone(),
            position: serde_json::to_value(village.position.clone()).map_err(|e| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "invalid village position payload for {}: {}",
                    village.id, e
                )))
            })?,
            buildings: serde_json::to_value(village.buildings()).map_err(|e| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "invalid village buildings payload for {}: {}",
                    village.id, e
                )))
            })?,
            production: serde_json::to_value(village.production.clone()).map_err(|e| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "invalid village production payload for {}: {}",
                    village.id, e
                )))
            })?,
            stocks: serde_json::to_value(village.stocks()).map_err(|e| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "invalid village stocks payload for {}: {}",
                    village.id, e
                )))
            })?,
            smithy_upgrades: serde_json::to_value(village.smithy()).map_err(|e| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "invalid village smithy payload for {}: {}",
                    village.id, e
                )))
            })?,
            academy_research: serde_json::to_value(village.academy_research()).map_err(|e| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "invalid village academy research payload for {}: {}",
                    village.id, e
                )))
            })?,
            population: i32::try_from(village.population).map_err(|_| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "village population overflow for {}: {}",
                    village.id, village.population
                )))
            })?,
            loyalty: i16::from(village.loyalty()),
            is_capital: village.is_capital,
            culture_points: i32::try_from(village.culture_points).map_err(|_| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "village culture_points overflow for {}: {}",
                    village.id, village.culture_points
                )))
            })?,
            culture_points_production: i32::try_from(village.culture_points_production).map_err(
                |_| {
                    ApplicationError::Db(DbError::Transaction(format!(
                        "village culture_points_production overflow for {}: {}",
                        village.id, village.culture_points_production
                    )))
                },
            )?,
            created_at: jiff::Timestamp::now(),
            updated_at: jiff::Timestamp::now(),
            parent_village_id: village.parent_village_id.map(|id| id as i32),
        })
    }
}
