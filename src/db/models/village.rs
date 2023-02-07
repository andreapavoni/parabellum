use chrono::{DateTime, Utc};
use ormlite::model::*;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use std::collections::HashMap;
use uuid::Uuid;

use crate::game::models::{
    army::Army,
    buildings::Building,
    map::Oasis,
    village::{StockCapacity, Village as GameVillage, VillageProduction},
    {SmithyUpgrades, Tribe},
};

#[derive(Model, Serialize, Deserialize, Debug, Clone)]
#[ormlite(table = "villages")]
pub struct Village {
    #[ormlite(primary_key)]
    pub id: u32,
    pub name: String,
    pub player_id: Uuid,
    pub valley_id: u32,
    pub tribe: Json<Tribe>,
    pub buildings: Json<HashMap<u8, Building>>,
    pub oases: Json<Vec<Oasis>>,
    pub population: u32,
    pub army: Json<Army>,
    pub reinforcements: Json<Vec<Army>>,
    pub loyalty: u8,
    pub production: Json<VillageProduction>,
    pub is_capital: bool,
    pub smithy: Json<SmithyUpgrades>,
    pub stocks: Json<StockCapacity>,
    pub updated_at: DateTime<Utc>,
}

impl From<Village> for GameVillage {
    fn from(v: Village) -> Self {
        Self {
            id: v.id,
            name: v.name,
            player_id: v.player_id,
            valley_id: v.valley_id,
            tribe: v.tribe.as_ref().clone(),
            buildings: v.buildings.as_ref().clone(),
            oases: v.oases.as_ref().clone(),
            population: v.population,
            army: v.army.as_ref().clone(),
            reinforcements: v.reinforcements.as_ref().clone(),
            loyalty: v.loyalty,
            production: v.production.as_ref().clone(),
            is_capital: v.is_capital,
            smithy: v.smithy.as_ref().clone(),
            stocks: v.stocks.as_ref().clone(),
            updated_at: v.updated_at,
        }
    }
}

impl From<GameVillage> for Village {
    fn from(v: GameVillage) -> Self {
        Self {
            id: v.id,
            name: v.name,
            player_id: v.player_id,
            valley_id: v.valley_id,
            tribe: Json(v.tribe),
            buildings: Json(v.buildings.clone()),
            oases: Json(v.oases.clone()),
            population: v.population,
            army: Json(v.army.clone()),
            reinforcements: Json(v.reinforcements.clone()),
            loyalty: v.loyalty,
            production: Json(v.production.clone()),
            is_capital: v.is_capital,
            smithy: Json(v.smithy.clone()),
            stocks: Json(v.stocks.clone()),
            updated_at: Utc::now(),
        }
    }
}
