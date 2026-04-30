//! ES integration-test fixtures.
//!
//! Fixture policy:
//! - initialize village streams through ES commands (`found_village`)
//! - seed resources through utility command (`set_village_resources`)
//! - keep scheduler tests deterministic by deriving due times from queued actions
use parabellum_game::models::{buildings::Building, village::VillageBuilding};
use parabellum_types::army::TroopSet;
use parabellum_types::{
    buildings::{BuildingGroup, BuildingName},
    common::ResourceGroup,
    map::Position,
    tribe::Tribe,
};
use serde_json::json;
use tokio::sync::Mutex;
use tokio::sync::OnceCell;
use uuid::Uuid;

use crate::es::VillageEsService;
use crate::establish_test_connection_pool;

static MIGRATIONS_ONCE: OnceCell<()> = OnceCell::const_new();
static TEST_DB_MUTEX: Mutex<()> = Mutex::const_new(());

pub async fn setup_pool() -> sqlx::PgPool {
    let pool = establish_test_connection_pool()
        .await
        .expect("TEST_DATABASE_URL connection must be available");
    MIGRATIONS_ONCE
        .get_or_init(|| async {
            sqlx::migrate!("../migrations")
                .run(&pool)
                .await
                .expect("failed to run test migrations");
        })
        .await;
    reset_tables(&pool).await;
    pool
}

pub async fn with_test_pool<T, F, Fut>(f: F) -> T
where
    F: FnOnce(sqlx::PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    // DB mutex guarantees isolated integration-test state for shared IDs.
    let _guard = TEST_DB_MUTEX.lock().await;
    let pool = setup_pool().await;
    f(pool).await
}

pub async fn reset_tables(pool: &sqlx::PgPool) {
    sqlx::query("DELETE FROM rm_village_movements")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM rm_scheduled_actions")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM rm_village")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM es_snapshots")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM es_events")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM villages")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM players")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users")
        .execute(pool)
        .await
        .unwrap();
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
    sqlx::query(
        "INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
    )
        .bind(user_id)
        .bind(format!("{user_id}@test.local"))
        .bind("hash")
        .execute(pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO players (id, username, tribe, user_id, culture_points) VALUES ($1, $2, 'Roman', $3, 0) ON CONFLICT (id) DO NOTHING",
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
        "warehouse_capacity": 800, "granary_capacity": 800, "lumber": 20000, "clay": 20000, "iron": 20000, "crop": 20000
    }))
    .bind(json!({}))
    .bind(json!({}))
    .execute(pool)
    .await
    .unwrap();
}

pub fn stocks_for_training() -> serde_json::Value {
    json!({
        "warehouse_capacity": 800,
        "granary_capacity": 800,
        "lumber": 2000,
        "clay": 2000,
        "iron": 2000,
        "crop": 2000
    })
}

pub fn stocks_for_research() -> serde_json::Value {
    json!({
        "warehouse_capacity": 80000,
        "granary_capacity": 80000,
        "lumber": 80000,
        "clay": 80000,
        "iron": 80000,
        "crop": 80000
    })
}

pub fn main_building(level: u8) -> VillageBuilding {
    VillageBuilding {
        slot_id: 19,
        building: Building {
            name: BuildingName::MainBuilding,
            group: BuildingGroup::Infrastructure,
            value: 0,
            population: 0,
            culture_points: 0,
            level,
        },
    }
}

pub fn barracks(level: u8) -> VillageBuilding {
    VillageBuilding {
        slot_id: 20,
        building: Building {
            name: BuildingName::Barracks,
            group: BuildingGroup::Military,
            value: 1000,
            population: 0,
            culture_points: 0,
            level,
        },
    }
}

pub fn smithy(level: u8) -> VillageBuilding {
    VillageBuilding {
        slot_id: 23,
        building: Building {
            name: BuildingName::Smithy,
            group: BuildingGroup::Infrastructure,
            value: 0,
            population: 0,
            culture_points: 0,
            level,
        },
    }
}

pub fn academy(level: u8) -> VillageBuilding {
    VillageBuilding {
        slot_id: 22,
        building: Building {
            name: BuildingName::Academy,
            group: BuildingGroup::Infrastructure,
            value: 0,
            population: 0,
            culture_points: 0,
            level,
        },
    }
}

pub fn warehouse(level: u8) -> VillageBuilding {
    let building = Building::new(BuildingName::Warehouse, 1)
        .at_level(level, 1)
        .expect("warehouse building data should be available for fixture");
    VillageBuilding {
        slot_id: 26,
        building,
    }
}

pub fn granary(level: u8) -> VillageBuilding {
    let building = Building::new(BuildingName::Granary, 1)
        .at_level(level, 1)
        .expect("granary building data should be available for fixture");
    VillageBuilding {
        slot_id: 27,
        building,
    }
}

pub async fn setup_scheduler_village(
    pool: &sqlx::PgPool,
    service: &VillageEsService,
    player_id: Uuid,
    user_id: Uuid,
    village_id: u32,
    village_name: &str,
    position: Position,
    tribe: Tribe,
    buildings: Vec<VillageBuilding>,
    stationed_units: TroopSet,
    stocks: serde_json::Value,
) {
    // Seed legacy source rows required by rm_village refresh and projector lookups.
    seed_player_and_village(
        pool,
        player_id,
        user_id,
        village_id as i32,
        village_name,
        position.x,
        position.y,
    )
    .await;

    sqlx::query(
        r#"
        UPDATE villages
        SET stocks = $1::jsonb
        WHERE id = $2
        "#,
    )
    .bind(stocks.clone())
    .bind(village_id as i32)
    .execute(pool)
    .await
    .unwrap();

    service
        .found_village(
            village_id,
            &parabellum_app::villages::FoundVillage {
                village_name: village_name.to_string(),
                position,
                tribe,
                player_id,
                stationed_units,
                buildings,
            },
        )
        .await
        .unwrap();
    // Apply explicit resource target through the ES utility command.
    let resources = ResourceGroup::new(
        stocks["lumber"].as_u64().unwrap_or(0) as u32,
        stocks["clay"].as_u64().unwrap_or(0) as u32,
        stocks["iron"].as_u64().unwrap_or(0) as u32,
        stocks["crop"].as_u64().unwrap_or(0) as u32,
    );
    service
        .set_village_resources(
            village_id,
            &parabellum_app::villages::SetVillageResources {
                player_id,
                resources,
            },
        )
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
        buildings: vec![main_building(1), rally_point(1)],
    }
}
