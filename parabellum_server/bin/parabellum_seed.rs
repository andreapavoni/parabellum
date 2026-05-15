use std::{fs, path::PathBuf};

use parabellum_app::auth::hash_password;
use parabellum_app::config::Config;
use parabellum_app::villages::{FoundVillage, SetVillageResources};
use parabellum_infra::{bootstrap_world_map, es::VillageEsService, establish_connection_pool};
use parabellum_game::models::map::{MapFieldTopology, MapQuadrant};
use parabellum_game::models::{buildings::Building, village::VillageBuilding};
use parabellum_types::buildings::BuildingName;
use parabellum_types::common::ResourceGroup;
use parabellum_types::errors::{ApplicationError, DbError};
use parabellum_types::map::{Position, ValleyTopology};
use parabellum_types::tribe::Tribe;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedFile {
    username: String,
    #[serde(default = "default_tribe")]
    tribe: Tribe,
    #[serde(default = "default_quadrant")]
    quadrant: SeedQuadrant,
    #[serde(default = "default_resource_fields_target_level")]
    resource_fields_target_level: u8,
    buildings: Vec<SeedBuilding>,
    position: Option<Position>,
    resources: Option<ResourceGroup>,
    speed: Option<i8>,
    village_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SeedBuilding {
    #[serde(rename = "slotId")]
    slot_id: u8,
    name: BuildingName,
    #[serde(default = "default_building_level")]
    level: u8,
}

#[derive(Debug, Clone, Deserialize)]
enum SeedQuadrant {
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl SeedQuadrant {
    fn as_domain(&self) -> MapQuadrant {
        match self {
            Self::NorthEast => MapQuadrant::NorthEast,
            Self::SouthEast => MapQuadrant::SouthEast,
            Self::SouthWest => MapQuadrant::SouthWest,
            Self::NorthWest => MapQuadrant::NorthWest,
        }
    }
}

fn default_building_level() -> u8 {
    1
}

fn default_resource_fields_target_level() -> u8 {
    0
}

fn default_tribe() -> Tribe {
    Tribe::Roman
}

fn default_quadrant() -> SeedQuadrant {
    SeedQuadrant::NorthEast
}

fn seed_inputs_from_args() -> (PathBuf, Option<String>) {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let default_path = PathBuf::from("seed/default.json");

    match args.as_slice() {
        [] => (default_path, None),
        [one] if one.ends_with(".json") => (PathBuf::from(one), None),
        [one] => (default_path, Some(one.clone())),
        [username, path] => (PathBuf::from(path), Some(username.clone())),
        _ => (
            default_path,
            args.first().cloned(), // best-effort fallback
        ),
    }
}

fn village_buildings(config_speed: i8, seed: &SeedFile) -> Result<Vec<VillageBuilding>, ApplicationError> {
    let speed = seed.speed.unwrap_or(config_speed);
    let mut buildings = Vec::with_capacity(seed.buildings.len());
    for item in &seed.buildings {
        let b = Building::new(item.name.clone(), speed)
            .at_level(item.level, speed)
            .map_err(ApplicationError::from)?;
        buildings.push(VillageBuilding {
            slot_id: item.slot_id,
            building: b,
        });
    }
    Ok(buildings)
}

fn topology_resource_buildings(
    topology: &ValleyTopology,
    speed: i8,
    level: u8,
) -> Result<Vec<VillageBuilding>, ApplicationError> {
    let mut slot_id: u8 = 1;
    let mut out = Vec::with_capacity(18);

    let mut push_n = |name: BuildingName, count: u8| -> Result<(), ApplicationError> {
        for _ in 0..count {
            out.push(VillageBuilding {
                slot_id,
                building: Building::new(name.clone(), speed)
                    .at_level(level, speed)
                    .map_err(ApplicationError::from)?,
            });
            slot_id = slot_id.saturating_add(1);
        }
        Ok(())
    };

    push_n(BuildingName::Woodcutter, topology.lumber())?;
    push_n(BuildingName::ClayPit, topology.clay())?;
    push_n(BuildingName::IronMine, topology.iron())?;
    push_n(BuildingName::Cropland, topology.crop())?;
    Ok(out)
}

fn build_identity(seed: &SeedFile, username_override: Option<&str>) -> (String, String, String) {
    let username = username_override
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| seed.username.clone());
    let email = format!("{username}@example.com");
    let village_name = seed
        .village_name
        .clone()
        .unwrap_or_else(|| format!("{username}'s Village"));
    (username, email, village_name)
}

async fn select_unoccupied_valley(
    pool: &sqlx::PgPool,
    quadrant: &MapQuadrant,
) -> Result<(u32, Position), ApplicationError> {
    let query = match quadrant {
        MapQuadrant::NorthEast => {
            "SELECT id, position FROM rm_map_fields WHERE village_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int > 0 AND topology = '{\"Valley\":[4,4,4,6]}'::jsonb ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
        }
        MapQuadrant::SouthEast => {
            "SELECT id, position FROM rm_map_fields WHERE village_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int < 0 AND topology = '{\"Valley\":[4,4,4,6]}'::jsonb ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
        }
        MapQuadrant::SouthWest => {
            "SELECT id, position FROM rm_map_fields WHERE village_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int < 0 AND topology = '{\"Valley\":[4,4,4,6]}'::jsonb ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
        }
        MapQuadrant::NorthWest => {
            "SELECT id, position FROM rm_map_fields WHERE village_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int > 0 AND topology = '{\"Valley\":[4,4,4,6]}'::jsonb ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
        }
    };

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
    let row: (i32, sqlx::types::Json<Position>) = sqlx::query_as(query)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
    tx.commit()
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
    Ok((row.0 as u32, row.1.0))
}

async fn load_valley_topology(
    pool: &sqlx::PgPool,
    field_id: u32,
) -> Result<ValleyTopology, ApplicationError> {
    let topo: sqlx::types::Json<MapFieldTopology> =
        sqlx::query_scalar("SELECT topology FROM rm_map_fields WHERE id = $1")
            .bind(field_id as i32)
            .fetch_one(pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
    match topo.0 {
        MapFieldTopology::Valley(v) => Ok(v),
        _ => Err(ApplicationError::Unknown(format!(
            "map field {} is not a valley",
            field_id
        ))),
    }
}

async fn ensure_migrations_and_map(
    pool: &sqlx::PgPool,
    config: &Config,
) -> Result<(), ApplicationError> {
    sqlx::migrate!("../migrations")
        .run(pool)
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

    // Defensive cleanup for interrupted/manual dev runs:
    // rm_map_fields may contain occupied references whose rm_village row does not exist.
    sqlx::query(
        r#"
        UPDATE rm_map_fields mf
        SET village_id = NULL,
            player_id = NULL
        WHERE mf.village_id IS NOT NULL
          AND NOT EXISTS (
            SELECT 1
            FROM rm_village rv
            WHERE rv.village_id = mf.village_id
          )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

    let _ = bootstrap_world_map(pool, config.world_size).await?;
    Ok(())
}

async fn ensure_map_field_is_free(pool: &sqlx::PgPool, village_id: u32) -> Result<(), ApplicationError> {
    let occupied: bool =
        sqlx::query_scalar("SELECT village_id IS NOT NULL FROM rm_map_fields WHERE id = $1")
            .bind(village_id as i32)
            .fetch_one(pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
    if occupied {
        return Err(ApplicationError::Unknown(format!(
            "target map field {} is already occupied",
            village_id
        )));
    }
    Ok(())
}

async fn create_identity(
    pool: &sqlx::PgPool,
    username: &str,
    email: &str,
    tribe: &Tribe,
) -> Result<(Uuid, Uuid), ApplicationError> {
    let existing_user_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

    let user_id = if let Some(id) = existing_user_id {
        id
    } else {
        sqlx::query_scalar("INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id")
            .bind(email)
            .bind(hash_password(username)?)
            .fetch_one(pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
    };

    let existing_player_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM players WHERE user_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

    let player_id = if let Some(id) = existing_player_id {
        id
    } else {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO players (id, username, tribe, user_id, culture_points) VALUES ($1, $2, $3::tribe, $4, 0)",
        )
        .bind(id)
        .bind(username)
        .bind(format!("{:?}", tribe))
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        id
    };

    Ok((user_id, player_id))
}

async fn reserve_map_field(
    pool: &sqlx::PgPool,
    village_id: u32,
    player_id: Uuid,
) -> Result<(), ApplicationError> {
    let updated = sqlx::query(
        "UPDATE rm_map_fields SET village_id = $1, player_id = $2 WHERE id = $1 AND village_id IS NULL",
    )
    .bind(village_id as i32)
    .bind(player_id)
    .execute(pool)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
    .rows_affected();
    if updated != 1 {
        return Err(ApplicationError::Unknown(format!(
            "cannot reserve selected map field {}",
            village_id
        )));
    }
    Ok(())
}


#[tokio::main]
async fn main() -> Result<(), ApplicationError> {
    let (path, username_override) = seed_inputs_from_args();
    let raw = fs::read_to_string(&path)
        .map_err(|e| ApplicationError::Unknown(format!("cannot read {}: {e}", path.display())))?;
    let seed: SeedFile = serde_json::from_str(&raw)
        .map_err(|e| ApplicationError::Unknown(format!("invalid seed JSON: {e}")))?;

    let config = Config::from_env();
    let pool = establish_connection_pool().await?;
    let (username, email, village_name) = build_identity(&seed, username_override.as_deref());

    ensure_migrations_and_map(&pool, &config).await?;

    let (village_id, village_position) = if let Some(pos) = seed.position.clone() {
        (pos.to_id(config.world_size as i32), pos)
    } else {
        select_unoccupied_valley(&pool, &seed.quadrant.as_domain()).await?
    };

    ensure_map_field_is_free(&pool, village_id).await?;
    let (_user_id, player_id) = create_identity(&pool, &username, &email, &seed.tribe).await?;

    let speed = seed.speed.unwrap_or(config.speed);
    let topology = load_valley_topology(&pool, village_id).await?;
    let mut resource_buildings =
        topology_resource_buildings(&topology, speed, seed.resource_fields_target_level)?;
    let mut extra_buildings = village_buildings(speed, &seed)?
        .into_iter()
        .filter(|b| b.slot_id >= 19)
        .collect::<Vec<_>>();
    resource_buildings.append(&mut extra_buildings);
    let buildings = resource_buildings;

    let service = VillageEsService::new(pool.clone());
    service
        .found_village(
            village_id,
            &FoundVillage {
                village_name: seed
                    .village_name
                    .unwrap_or(village_name.clone()),
                position: village_position.clone(),
                tribe: seed.tribe.clone(),
                player_id,
                buildings,
            },
        )
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
    reserve_map_field(&pool, village_id, player_id).await?;
    service
        .set_village_resources(
            village_id,
            &SetVillageResources {
                player_id,
                resources: seed
                    .resources
                    .unwrap_or_else(|| ResourceGroup::new(80_000, 80_000, 80_000, 80_000)),
            },
        )
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

    println!(
        "Seed completed from {}:\n  username={}\n  email={}\n  password={}\n  village='{}'\n  village_id={}\n  position=({}, {})",
        path.display(),
        username,
        email,
        username,
        village_name,
        village_id,
        village_position.x,
        village_position.y
    );

    Ok(())
}
