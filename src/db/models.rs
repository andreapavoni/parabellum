use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::game::models::{
    army::TroopSet,
    map::{MapFieldTopology, Position},
    village::{StockCapacity, VillageBuilding, VillageProduction},
    SmithyUpgrades,
};

use super::schema::{armies, map_fields, players, villages};
use super::utils::JsonbWrapper;
use crate::impl_jsonb_for;

impl_jsonb_for!(StockCapacity);
impl_jsonb_for!(VillageProduction);
impl_jsonb_for!(SmithyUpgrades);
impl_jsonb_for!(Vec<VillageBuilding>);

#[derive(DbEnum, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[ExistingTypePath = "crate::db::schema::sql_types::Tribe"]
pub enum Tribe {
    #[db_rename = "Roman"]
    Roman,
    #[db_rename = "Gaul"]
    Gaul,
    #[db_rename = "Teuton"]
    Teuton,
    #[db_rename = "Natar"]
    Natar,
    #[db_rename = "Nature"]
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
    pub id: i32,
    pub player_id: Uuid,
    pub name: String,
    pub pos_x: i32,
    pub pos_y: i32,
    pub buildings: JsonbWrapper<Vec<VillageBuilding>>,
    pub production: JsonbWrapper<VillageProduction>,
    pub stocks: JsonbWrapper<StockCapacity>,
    pub smithy_upgrades: JsonbWrapper<SmithyUpgrades>,
    pub population: i32,
    pub loyalty: i16,
    pub is_capital: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = villages)]
pub struct NewVillage<'a> {
    pub id: i32,
    pub player_id: Uuid,
    pub name: &'a str,
    pub pos_x: i32,
    pub pos_y: i32,
    pub buildings: JsonbWrapper<Vec<VillageBuilding>>,
    pub production: JsonbWrapper<VillageProduction>,
    pub stocks: JsonbWrapper<StockCapacity>,
    pub smithy_upgrades: JsonbWrapper<SmithyUpgrades>,
    pub population: i32,
    pub loyalty: i16,
    pub is_capital: bool,
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
    pub id: i32,
    pub village_id: Option<i32>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::run_test_with_transaction;
    use crate::db::test_helpers::*;
    use crate::game::models::map::OasisTopology;

    #[test]
    fn test_factories_with_defaults() {
        run_test_with_transaction(|conn| {
            let player = player_factory(conn, Default::default());
            assert!(player.username.starts_with("user_"));
            assert_eq!(player.tribe, Tribe::Roman);

            let village = village_factory(conn, Default::default());
            assert_eq!(village.name, "Factory Village");

            let army = army_factory(conn, Default::default());
            assert_eq!(army.units.0[0], 10);

            let field_default = map_field_factory(conn, Default::default());
            assert!(field_default.id != 0);

            Ok(())
        });
    }

    #[test]
    fn test_factories_with_overrides() {
        run_test_with_transaction(|conn| {
            let player = player_factory(
                conn,
                PlayerFactoryOptions {
                    username: Some("Dino"),
                    tribe: Some(Tribe::Gaul),
                },
            );
            assert_eq!(player.username, "Dino");
            assert_eq!(player.tribe, Tribe::Gaul);

            let village = village_factory(
                conn,
                VillageFactoryOptions {
                    player_id: Some(player.id),
                    name: Some("Dino's Village"),
                },
            );
            assert_eq!(village.player_id, player.id);
            assert_eq!(village.name, "Dino's Village");

            let custom_units: TroopSet = [100, 100, 0, 0, 0, 0, 0, 0, 0, 0];
            let army = army_factory(
                conn,
                ArmyFactoryOptions {
                    player_id: Some(player.id),
                    village_id: Some(village.id),
                    units: Some(custom_units),
                },
            );
            assert_eq!(army.player_id, player.id);
            assert_eq!(army.village_id, village.id);
            assert_eq!(army.units.0, custom_units);

            let custom_pos = Position { x: 123, y: -45 };
            let custom_topo = MapFieldTopology::Oasis(OasisTopology::Crop50);

            let field_custom = map_field_factory(
                conn,
                MapFieldFactoryOptions {
                    position: Some(custom_pos.clone()),
                    topology: Some(custom_topo.clone()),
                },
            );

            assert_eq!(field_custom.position.0, custom_pos);
            assert_eq!(field_custom.topology.0, custom_topo);

            Ok(())
        });
    }
}
