use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::schema::map_fields;
use crate::game::models::army::TroopSet;
use crate::game::models::map::{MapFieldTopology, Position};
use crate::game::models::village::{StockCapacity, VillageBuilding, VillageProduction};
use crate::game::models::SmithyUpgrades;

use super::schema::{armies, players, villages};
use super::utils::JsonbWrapper;
use crate::impl_jsonb_for;

impl_jsonb_for!(StockCapacity);
impl_jsonb_for!(VillageProduction);
impl_jsonb_for!(SmithyUpgrades);
impl_jsonb_for!(Vec<VillageBuilding>);

#[derive(DbEnum, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[ExistingTypePath = "crate::db::schema::sql_types::Tribe"]
pub enum Tribe {
    Roman,
    Gaul,
    Teuton,
    Natar,
    Nature,
}

#[derive(Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = players)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
    pub tribe: Tribe,
}

#[derive(Insertable)]
#[diesel(table_name = players)]
pub struct NewPlayer<'a> {
    pub id: Uuid,
    pub username: &'a str,
    pub tribe: Tribe,
}

#[derive(Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = villages)]
pub struct Village {
    pub id: u32,
    pub player_id: Uuid,
    pub name: String,
    pub pos_x: i32,
    pub pos_y: i32,
    pub loyalty: i16,
    pub is_capital: bool,
    pub updated_at: chrono::NaiveDateTime,
    pub buildings: JsonbWrapper<Vec<VillageBuilding>>,
    pub production: JsonbWrapper<VillageProduction>,
    pub stocks: JsonbWrapper<StockCapacity>,
    pub smithy_upgrades: JsonbWrapper<SmithyUpgrades>,
    pub population: u32,
}

#[derive(Insertable)]
#[diesel(table_name = villages)]
pub struct NewVillage<'a> {
    pub id: i32,
    pub player_id: Uuid,
    pub name: &'a str,
    pub pos_x: i32,
    pub pos_y: i32,
    pub loyalty: i16,
    pub is_capital: bool,
    pub buildings: JsonbWrapper<Vec<VillageBuilding>>,
    pub production: JsonbWrapper<VillageProduction>,
    pub stocks: JsonbWrapper<StockCapacity>,
    pub smithy_upgrades: JsonbWrapper<SmithyUpgrades>,
    pub population: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = armies)]
pub struct Army {
    pub id: Uuid,
    pub village_id: i32,
    pub current_map_field_id: i32, // Oasis or village
    pub hero_id: Option<Uuid>,
    pub units: JsonbWrapper<TroopSet>,
    pub smithy: JsonbWrapper<SmithyUpgrades>,
    pub tribe: Tribe,
    pub player_id: Uuid,
}

#[derive(Insertable)]
#[diesel(table_name = armies)]
pub struct NewArmy<'a> {
    pub id: Uuid,
    pub village_id: i32,
    pub current_map_field_id: i32, // Oasis or village
    pub hero_id: Option<Uuid>,
    pub units: &'a JsonbWrapper<TroopSet>,
    pub smithy: &'a JsonbWrapper<SmithyUpgrades>,
    pub tribe: Tribe,
    pub player_id: Uuid,
}

#[derive(Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = map_fields)]
pub struct MapField {
    pub id: u32,
    pub village_id: Option<u32>,
    pub player_id: Option<Uuid>,
    pub position: JsonbWrapper<Position>,
    pub topology: JsonbWrapper<MapFieldTopology>,
}

#[derive(Insertable)]
#[diesel(table_name = map_fields)]
pub struct NewMapField<'a> {
    pub id: i32,
    pub village_id: Option<i32>,
    pub player_id: Option<Uuid>,
    pub position: &'a JsonbWrapper<Position>,
    pub topology: &'a JsonbWrapper<MapFieldTopology>,
}
