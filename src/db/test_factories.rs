use rand::Rng;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;

use crate::game::models::{
    army::TroopSet,
    map::{MapFieldTopology, Position, Valley, ValleyTopology},
    smithy::SmithyUpgrades,
    village::{self, VillageBuilding, VillageProduction, VillageStocks},
};

use super::models::*;

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
    pub stocks: Option<VillageStocks>,
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

pub async fn player_factory(conn: &mut PgConnection, options: PlayerFactoryOptions<'_>) -> Player {
    let default_username: String = format!("user_{}", rand::thread_rng().r#gen::<u32>());
    let id = options.id.unwrap_or_else(Uuid::new_v4);
    let username = options.username.unwrap_or(&default_username);
    let tribe = options.tribe.unwrap_or(Tribe::Roman);

    sqlx::query_as!(
        Player,
        r#"
        INSERT INTO players (id, username, tribe)
        VALUES ($1, $2, $3)
        RETURNING id, username, tribe as "tribe: _"
        "#,
        id,
        username,
        tribe as _
    )
    .fetch_one(conn)
    .await
    .expect("Failed to create player with factory")
}

pub async fn village_factory(
    conn: &mut PgConnection,
    options: VillageFactoryOptions<'_>,
    world_size: i32,
) -> Village {
    let tmp_player = player_factory(conn, Default::default()).await;
    let player_id = options.player_id.unwrap_or(tmp_player.id);

    let position = match options.position {
        Some(position) => position.clone(),
        None => Position {
            x: rand::thread_rng().gen_range(-world_size..world_size),
            y: rand::thread_rng().gen_range(-world_size..world_size),
        },
    };

    let valley = Valley::new(position.clone(), ValleyTopology(4, 4, 4, 6));
    let village = village::Village::new(
        options.name.unwrap_or("Factory Village").to_string(),
        &valley,
        &tmp_player.clone().into(),
        true,
        world_size,
    );

    sqlx::query_as!(
        Village,
        r#"
        INSERT INTO villages (id, player_id, name, position, buildings, production, stocks, smithy_upgrades, population, loyalty, is_capital, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        RETURNING *
        "#,
        village.id as i32,
        player_id,
        village.name,
        Json(&position) as _,
        Json(&village.buildings) as _,
        Json(&village.production) as _,
        Json(&village.stocks) as _,
        Json(&village.smithy) as _,
        village.population as i32,
        village.loyalty as i16,
        village.is_capital,
        village.updated_at,
        village.updated_at
    )
    .fetch_one(conn)
    .await
    .expect("Failed to create village with factory")
}

pub async fn army_factory(
    conn: &mut PgConnection,
    options: ArmyFactoryOptions,
    world_size: i32,
) -> Army {
    let (owner_id, home_village_id) = match (options.player_id, options.village_id) {
        (Some(p_id), Some(v_id)) => (p_id, v_id),
        (Some(p_id), None) => {
            let village = village_factory(
                conn,
                VillageFactoryOptions {
                    player_id: Some(p_id),
                    ..Default::default()
                },
                world_size,
            )
            .await;
            (village.player_id, village.id)
        }
        _ => {
            let village = village_factory(conn, Default::default(), world_size).await;
            (village.player_id, village.id)
        }
    };

    let units_data = options.units.unwrap_or([10, 5, 0, 0, 0, 0, 0, 0, 0, 0]);
    let smithy_data: SmithyUpgrades = options.smithy.unwrap_or([1, 1, 0, 0, 0, 0, 0, 0]);

    sqlx::query_as!(
        Army,
        r#"
        INSERT INTO armies (id, village_id, current_map_field_id, hero_id, units, smithy, tribe, player_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, village_id, current_map_field_id, hero_id, units, smithy, tribe as "tribe: _", player_id
        "#,
        Uuid::new_v4(),
        home_village_id,
        home_village_id,
        None::<Uuid>,
        Json(&units_data) as _,
        Json(&smithy_data) as _,
        Tribe::Teuton as _,
        owner_id
    )
    .fetch_one(conn)
    .await
    .expect("Failed to create army with factory")
}

pub async fn map_field_factory(
    conn: &mut PgConnection,
    options: MapFieldFactoryOptions,
) -> MapField {
    let default_pos = Position {
        x: rand::thread_rng().r#gen(),
        y: rand::thread_rng().r#gen(),
    };
    let default_topo = MapFieldTopology::Valley(ValleyTopology(4, 4, 4, 6));
    let position = options.position.unwrap_or(default_pos);
    let topology = options.topology.unwrap_or(default_topo);
    let id: i32 = rand::thread_rng().gen_range(1..i32::MAX); // Ensure positive ID

    sqlx::query_as!(
        MapField,
        r#"
        INSERT INTO map_fields (id, village_id, player_id, position, topology)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
        id,
        options.village_id,
        options.player_id,
        Json(&position) as _,
        Json(&topology) as _,
    )
    .fetch_one(conn)
    .await
    .expect("Failed to create map_field with factory")
}
