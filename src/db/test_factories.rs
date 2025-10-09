use diesel::prelude::*;
use rand::Rng;
use uuid::Uuid;

use crate::game::models::{
    army::TroopSet,
    map::{MapFieldTopology, Position, ValleyTopology},
    village::{StockCapacity, VillageBuilding, VillageProduction},
    SmithyUpgrades,
};

use super::{models::*, schema::*, utils::JsonbWrapper};

#[derive(Default)]
pub struct PlayerFactoryOptions<'a> {
    pub id: Option<Uuid>,
    pub username: Option<&'a str>,
    pub tribe: Option<Tribe>,
}

#[derive(Default)]
pub struct VillageFactoryOptions<'a> {
    pub player_id: Option<Uuid>,
    pub name: Option<&'a str>,
    pub position: Option<&'a Position>,
    pub buildings: Option<Vec<VillageBuilding>>,
    pub production: Option<VillageProduction>,
    pub stocks: Option<StockCapacity>,
    pub smithy_upgrades: Option<SmithyUpgrades>,
    pub population: i32,
    pub loyalty: i16,
    pub is_capital: bool,
}

#[derive(Default)]
pub struct ArmyFactoryOptions {
    pub id: Option<Uuid>,
    pub village_id: Option<i32>,
    pub current_map_field_id: Option<i32>, // Oasis or village
    pub hero_id: Option<Uuid>,
    pub units: Option<TroopSet>,
    pub smithy: Option<SmithyUpgrades>,
    pub tribe: Option<Tribe>,
    pub player_id: Option<Uuid>,
}

#[derive(Default, Clone)]
pub struct MapFieldFactoryOptions {
    pub village_id: Option<i32>,
    pub player_id: Option<Uuid>,
    pub position: Option<Position>,
    pub topology: Option<MapFieldTopology>,
}

pub fn player_factory(conn: &mut PgConnection, options: PlayerFactoryOptions) -> Player {
    let default_username: String = format!("user_{}", rand::thread_rng().gen::<u32>());
    let new_player = NewPlayer {
        id: options.id.unwrap_or(Uuid::new_v4()),
        username: options.username.unwrap_or(&default_username),
        tribe: options.tribe.unwrap_or(Tribe::Roman),
    };

    diesel::insert_into(players::table)
        .values(&new_player)
        .get_result(conn)
        .expect("Failed to create player with factory")
}

pub fn village_factory(conn: &mut PgConnection, options: VillageFactoryOptions) -> Village {
    let world_size = 100;
    let owner_id = options
        .player_id
        .unwrap_or_else(|| player_factory(conn, Default::default()).id);

    let position = match options.position {
        Some(position) => position.clone(),
        None => Position {
            x: rand::thread_rng().gen_range(-world_size..world_size),
            y: rand::thread_rng().gen_range(-world_size..world_size),
        },
    };

    let village_id = position.clone().to_id(world_size) as i32;

    let new_village = NewVillage {
        id: village_id,
        player_id: owner_id,
        name: options.name.unwrap_or("Factory Village"),
        position: JsonbWrapper(position.clone()),

        buildings: JsonbWrapper(vec![]),
        production: JsonbWrapper(Default::default()),
        stocks: JsonbWrapper(Default::default()),
        smithy_upgrades: JsonbWrapper([0; 10]),
        population: 10,
        loyalty: 100,
        is_capital: false,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    diesel::insert_into(villages::table)
        .values(&new_village)
        .get_result(conn)
        .expect("Failed to create village with factory")
}

pub fn army_factory(conn: &mut PgConnection, options: ArmyFactoryOptions) -> Army {
    let (owner_id, home_village_id) = match (options.player_id, options.village_id) {
        (Some(p_id), Some(v_id)) => (p_id, v_id),
        (Some(p_id), None) => {
            let village = village_factory(
                conn,
                VillageFactoryOptions {
                    player_id: Some(p_id),
                    ..Default::default()
                },
            );
            (village.player_id, village.id)
        }
        _ => {
            let village = village_factory(conn, Default::default());
            (village.player_id, village.id)
        }
    };

    let units_data = options.units.unwrap_or([10, 5, 0, 0, 0, 0, 0, 0, 0, 0]);
    let smithy_data: SmithyUpgrades = options.smithy.unwrap_or([1, 1, 0, 0, 0, 0, 0, 0, 0, 0]);

    let new_army = NewArmy {
        id: Uuid::new_v4(),
        village_id: home_village_id,
        current_map_field_id: home_village_id,
        hero_id: None,
        units: &JsonbWrapper(units_data),
        smithy: &JsonbWrapper(smithy_data.clone()),
        tribe: Tribe::Teuton,
        player_id: owner_id,
    };

    diesel::insert_into(armies::table)
        .values(&new_army)
        .get_result(conn)
        .expect("Failed to create army with factory")
}

pub fn map_field_factory(conn: &mut PgConnection, options: MapFieldFactoryOptions) -> MapField {
    let default_pos = Position {
        x: rand::thread_rng().gen(),
        y: rand::thread_rng().gen(),
    };
    let default_topo = MapFieldTopology::Valley(ValleyTopology(4, 4, 4, 6));

    let new_map_field = NewMapField {
        id: rand::thread_rng().gen(),
        village_id: None,
        player_id: None,
        position: &JsonbWrapper(options.position.unwrap_or(default_pos)),
        topology: &JsonbWrapper(options.topology.unwrap_or(default_topo)),
    };

    diesel::insert_into(map_fields::table)
        .values(&new_map_field)
        .get_result(conn)
        .expect("Failed to create map_field with factory")
}
