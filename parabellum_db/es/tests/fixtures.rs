use parabellum_game::models::{buildings::Building, village::VillageBuilding};
use parabellum_types::{
    buildings::{BuildingGroup, BuildingName},
    tribe::Tribe,
};
use serde_json::json;
use uuid::Uuid;

use crate::establish_test_connection_pool;

pub async fn setup_pool() -> Option<sqlx::PgPool> {
    let pool = establish_test_connection_pool().await.ok()?;
    if !table_exists(&pool, "es_events").await
        || !table_exists(&pool, "rm_village").await
        || !table_exists(&pool, "rm_scheduled_actions").await
    {
        eprintln!("Skipping test: ES/projected tables are not present in TEST_DATABASE_URL");
        return None;
    }
    reset_tables(&pool).await;
    Some(pool)
}

pub async fn table_exists(pool: &sqlx::PgPool, table_name: &str) -> bool {
    sqlx::query_scalar::<_, Option<String>>("SELECT to_regclass($1)")
        .bind(format!("public.{table_name}"))
        .fetch_one(pool)
        .await
        .ok()
        .flatten()
        .is_some()
}

pub async fn reset_tables(pool: &sqlx::PgPool) {
    if table_exists(pool, "rm_village_movements").await {
        sqlx::query("DELETE FROM rm_village_movements")
            .execute(pool)
            .await
            .unwrap();
    }
    if table_exists(pool, "rm_scheduled_actions").await {
        sqlx::query("DELETE FROM rm_scheduled_actions")
            .execute(pool)
            .await
            .unwrap();
    }
    if table_exists(pool, "rm_village").await {
        sqlx::query("DELETE FROM rm_village").execute(pool).await.unwrap();
    }
    if table_exists(pool, "es_snapshots").await {
        sqlx::query("DELETE FROM es_snapshots")
            .execute(pool)
            .await
            .unwrap();
    }
    if table_exists(pool, "es_events").await {
        sqlx::query("DELETE FROM es_events").execute(pool).await.unwrap();
    }
    if table_exists(pool, "villages").await {
        sqlx::query("DELETE FROM villages").execute(pool).await.unwrap();
    }
    if table_exists(pool, "players").await {
        sqlx::query("DELETE FROM players").execute(pool).await.unwrap();
    }
    if table_exists(pool, "users").await {
        sqlx::query("DELETE FROM users").execute(pool).await.unwrap();
    }
}

pub async fn seed_player_and_village(
    pool: &sqlx::PgPool,
    player_id: Uuid,
    user_id: Uuid,
    village_id: i32,
    village_name: &str,
    position_x: i32,
    position_y: i32,
) {
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(format!("{user_id}@test.local"))
        .bind("hash")
        .execute(pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO players (id, username, tribe, user_id, culture_points) VALUES ($1, $2, 'Roman', $3, 0)",
    )
    .bind(player_id)
    .bind(format!("player_{player_id}"))
    .bind(user_id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO villages (
            id, player_id, name, position, buildings, production, stocks, smithy_upgrades, academy_research,
            population, loyalty, is_capital, culture_points, culture_points_production, parent_village_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 2, 100, false, 0, 0, NULL)
        "#,
    )
    .bind(village_id)
    .bind(player_id)
    .bind(village_name)
    .bind(json!({ "x": position_x, "y": position_y }))
    .bind(json!([]))
    .bind(json!({
        "lumber": 0, "clay": 0, "iron": 0, "crop": 0,
        "upkeep": 0, "bonus": {"lumber": 0, "clay": 0, "iron": 0, "crop": 0},
        "effective": {"lumber": 0, "clay": 0, "iron": 0, "crop": 0}
    }))
    .bind(json!({
        "warehouse_capacity": 800, "granary_capacity": 800, "lumber": 0, "clay": 0, "iron": 0, "crop": 0
    }))
    .bind(json!({}))
    .bind(json!({}))
    .execute(pool)
    .await
    .unwrap();
}

pub fn rally_point(level: u8) -> VillageBuilding {
    VillageBuilding {
        slot_id: 39,
        building: Building {
            name: BuildingName::RallyPoint,
            group: BuildingGroup::Infrastructure,
            value: 0,
            population: 0,
            culture_points: 0,
            level,
        },
    }
}

pub fn building_level(
    buildings: &[VillageBuilding],
    slot_id: u8,
    name: BuildingName,
) -> Option<u8> {
    buildings
        .iter()
        .find(|b| b.slot_id == slot_id && b.building.name == name)
        .map(|b| b.building.level)
}

pub fn found_village_cmd(
    player_id: Uuid,
    village_name: &str,
    position: parabellum_types::map::Position,
) -> parabellum_app::villages::FoundVillage {
    parabellum_app::villages::FoundVillage {
        village_name: village_name.to_string(),
        position,
        tribe: Tribe::Roman,
        player_id,
        stationed_units: parabellum_types::army::TroopSet::new([20, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        buildings: vec![rally_point(1)],
    }
}
