//! ES integration-test fixtures.
//!
//! Fixture policy:
//! - initialize village streams through ES commands (`found_village`)
//! - seed resources through utility command (`set_village_resources`)
//! - keep scheduler tests deterministic by deriving due times from queued actions
use parabellum_game::models::{buildings::Building, village::VillageBuilding};
use parabellum_types::{
    buildings::{BuildingGroup, BuildingName},
    common::ResourceGroup,
    map::Position,
    tribe::Tribe,
};
use tokio::sync::Mutex;
use tokio::sync::OnceCell;
use uuid::Uuid;

use crate::es::VillageEsService;
use crate::es::repositories::PostgresArmyRepository;
use crate::establish_test_connection_pool;
use parabellum_app::villages::models::ScheduledActionStatus;
use parabellum_app::villages::repositories::ArmyRepository;
use parabellum_app::villages::{ResearchAcademy, SetVillageResources, TrainUnits};
use parabellum_types::army::UnitName;

static MIGRATIONS_ONCE: OnceCell<()> = OnceCell::const_new();
static TEST_DB_MUTEX: Mutex<()> = Mutex::const_new(());
const TEST_DB_ADVISORY_LOCK_KEY: i64 = 9_842_771;

pub async fn setup_pool() -> sqlx::PgPool {
    // Run embedded migrations once for the shared test database.
    let pool = establish_test_connection_pool()
        .await
        .expect("TEST_DATABASE_URL connection must be available");
    MIGRATIONS_ONCE
        .get_or_init(|| async {
            sqlx::migrate!("../migrations")
                .run(&pool)
                .await
                .expect("failed to run test migrations");
            crate::bootstrap_world_map(&pool, 100)
                .await
                .expect("failed to bootstrap rm_map_fields");
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
    // In-process serialization.
    let _guard = TEST_DB_MUTEX.lock().await;
    let pool = setup_pool().await;
    // Cross-process serialization (multiple `cargo test` processes sharing TEST_DATABASE_URL).
    let mut lock_conn = pool
        .acquire()
        .await
        .expect("failed to acquire test lock connection");
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(TEST_DB_ADVISORY_LOCK_KEY)
        .execute(&mut *lock_conn)
        .await
        .expect("failed to acquire test advisory lock");

    let result = f(pool).await;

    // Best-effort unlock; lock also auto-releases when `lock_conn` is dropped.
    let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(TEST_DB_ADVISORY_LOCK_KEY)
        .execute(&mut *lock_conn)
        .await;

    result
}

pub async fn reset_tables(pool: &sqlx::PgPool) {
    sqlx::query("DELETE FROM rm_report_reads")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM rm_reports")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM rm_marketplace_offers")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM rm_village_movements")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM rm_armies")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM rm_heroes")
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
    sqlx::query("UPDATE rm_map_fields SET village_id = NULL, player_id = NULL")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM es_snapshots")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM es_projector_offsets")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM es_events")
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

pub async fn seed_user_and_player(pool: &sqlx::PgPool) -> (Uuid, Uuid) {
    let user_id = Uuid::new_v4();
    let player_id = Uuid::new_v4();

    // Village rows are not seeded for ES tests; villages are created by `FoundVillage`.
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

    (user_id, player_id)
}

pub fn resources(lumber: u32, clay: u32, iron: u32, crop: u32) -> ResourceGroup {
    ResourceGroup::new(lumber, clay, iron, crop)
}

pub async fn process_due_until(
    service: &VillageEsService,
    until: chrono::DateTime<chrono::Utc>,
    limit: i64,
) -> usize {
    service.process_due_actions(until, limit).await.unwrap()
}

pub async fn refill_resources(
    service: &VillageEsService,
    village_id: u32,
    player_id: Uuid,
    stocks: ResourceGroup,
) {
    service
        .set_village_resources(
            village_id,
            &SetVillageResources {
                player_id,
                resources: stocks,
            },
        )
        .await
        .unwrap();
}

pub async fn research_and_complete(
    service: &VillageEsService,
    village_id: u32,
    player_id: Uuid,
    unit: UnitName,
    speed: i8,
    due_by: chrono::DateTime<chrono::Utc>,
    process_limit: i64,
) {
    service
        .research_academy(
            village_id,
            &ResearchAcademy {
                player_id,
                unit,
                speed,
            },
        )
        .await
        .unwrap();
    process_due_until(service, due_by, process_limit).await;
}

pub async fn train_and_complete(
    service: &VillageEsService,
    village_id: u32,
    player_id: Uuid,
    unit_idx: u8,
    building_name: BuildingName,
    quantity: i32,
    speed: i8,
    due_by: chrono::DateTime<chrono::Utc>,
    process_limit: i64,
) {
    service
        .train_units(
            village_id,
            &TrainUnits {
                player_id,
                unit_idx,
                building_name,
                quantity,
                speed,
            },
        )
        .await
        .unwrap();
    process_due_until(service, due_by, process_limit).await;
}

pub async fn home_units(pool: &sqlx::PgPool, village_id: u32, unit_idx: usize) -> u32 {
    PostgresArmyRepository::new(pool.clone())
        .get_home_army(village_id)
        .await
        .unwrap()
        .map(|army| army.units().get(unit_idx))
        .unwrap_or(0)
}

pub async fn stationed_units(pool: &sqlx::PgPool, village_id: u32, unit_idx: usize) -> u32 {
    PostgresArmyRepository::new(pool.clone())
        .list_stationed_armies(village_id)
        .await
        .unwrap()
        .iter()
        .map(|army| army.units().get(unit_idx))
        .sum()
}

pub async fn deployed_units(pool: &sqlx::PgPool, village_id: u32, unit_idx: usize) -> u32 {
    PostgresArmyRepository::new(pool.clone())
        .list_deployed_armies(village_id)
        .await
        .unwrap()
        .iter()
        .map(|army| army.units().get(unit_idx))
        .sum()
}

pub async fn village_owner(service: &VillageEsService, village_id: u32) -> Uuid {
    service.get_village(village_id).await.unwrap().player_id
}

pub async fn village_stocks(
    service: &VillageEsService,
    village_id: u32,
) -> parabellum_game::models::village::VillageStocks {
    service.get_village(village_id).await.unwrap().stocks
}

pub async fn village_busy_merchants(service: &VillageEsService, village_id: u32) -> u8 {
    service
        .get_village(village_id)
        .await
        .unwrap()
        .busy_merchants
}

pub async fn insert_corrupt_scheduled_action(
    pool: &sqlx::PgPool,
    status: ScheduledActionStatus,
) -> Uuid {
    let action_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO rm_scheduled_actions (id, action_type, execute_at, payload, status, updated_at)
        VALUES ($1, 'TrainUnit', NOW() - interval '10 minutes', '{}'::jsonb, $2::scheduled_action_status, NOW() - interval '10 minutes')
        "#,
    )
    .bind(action_id)
    .bind(status.to_string())
    .execute(pool)
    .await
    .unwrap();
    action_id
}

pub async fn scheduled_action_status_count(
    pool: &sqlx::PgPool,
    status: ScheduledActionStatus,
) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM rm_scheduled_actions WHERE status = $1::scheduled_action_status",
    )
        .bind(status.to_string())
        .fetch_one(pool)
        .await
        .unwrap()
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

pub fn marketplace(level: u8) -> VillageBuilding {
    let building = Building::new(BuildingName::Marketplace, 1)
        .at_level(level, 1)
        .expect("marketplace building data should be available for fixture");
    VillageBuilding {
        slot_id: 28,
        building,
    }
}

pub async fn setup_village_for_player(
    service: &VillageEsService,
    player_id: Uuid,
    village_name: &str,
    position: Position,
    tribe: Tribe,
    buildings: Vec<VillageBuilding>,
    resources: ResourceGroup,
) -> u32 {
    let village_id = position.to_id(100);
    service
        .found_village(
            village_id,
            &parabellum_app::villages::FoundVillage {
                village_name: village_name.to_string(),
                position,
                tribe,
                player_id,
                parent_village_id: None,
                buildings,
            },
        )
        .await
        .unwrap();

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
    village_id
}

pub async fn setup_village(
    pool: &sqlx::PgPool,
    service: &VillageEsService,
    village_name: &str,
    position: Position,
    tribe: Tribe,
    buildings: Vec<VillageBuilding>,
    resources: ResourceGroup,
) -> (Uuid, Uuid, u32) {
    // Seed identity rows only. Village state is initialized via FoundVillage.
    let (user_id, player_id) = seed_user_and_player(pool).await;

    let village_id = setup_village_for_player(
        service,
        player_id,
        village_name,
        position,
        tribe,
        buildings,
        resources,
    )
    .await;
    (user_id, player_id, village_id)
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
