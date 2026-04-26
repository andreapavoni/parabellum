use std::{fs::File, io::BufReader, path::PathBuf, sync::Arc};

use parabellum_app::{auth::hash_password, config::Config, uow::UnitOfWorkProvider};
use parabellum_db::{
    bootstrap_world_map, establish_connection_pool, uow::PostgresUnitOfWorkProvider,
};
use parabellum_game::models::{
    buildings::{Building, get_building_data},
    map::MapQuadrant,
    village::Village,
};
use parabellum_types::{
    buildings::BuildingName, common::Player, errors::ApplicationError, tribe::Tribe,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

const DEFAULT_SEED_PATH: &str = "seed/game.json";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedFile {
    user: SeedUser,
    village: Option<SeedVillage>,
    #[serde(default)]
    buildings: Vec<SeedBuilding>,
    #[serde(default = "default_resource_fields_target_level")]
    resource_fields_target_level: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedUser {
    username: String,
    email: String,
    password: String,
    tribe: Tribe,
    quadrant: SeedQuadrant,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedVillage {
    name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedBuilding {
    slot_id: u8,
    name: BuildingName,
    level: Option<u8>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
enum SeedQuadrant {
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl From<SeedQuadrant> for MapQuadrant {
    fn from(value: SeedQuadrant) -> Self {
        match value {
            SeedQuadrant::NorthEast => Self::NorthEast,
            SeedQuadrant::SouthEast => Self::SouthEast,
            SeedQuadrant::SouthWest => Self::SouthWest,
            SeedQuadrant::NorthWest => Self::NorthWest,
        }
    }
}

fn default_resource_fields_target_level() -> u8 {
    10
}

#[tokio::main]
async fn main() -> Result<(), ApplicationError> {
    let seed_path = parse_seed_path();
    let seed = load_seed_file(&seed_path)?;

    let config = Arc::new(Config::from_env());
    let db_pool = establish_connection_pool().await?;
    sqlx::migrate!("../migrations")
        .run(&db_pool)
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
    setup_world_map(&db_pool, &config).await?;

    let uow_provider = Arc::new(PostgresUnitOfWorkProvider::new(db_pool.clone()));
    let uow = uow_provider.tx().await?;
    let result = seed_game(&uow, &seed, &config).await;
    match result {
        Ok(summary) => {
            uow.commit().await?;
            println!(
                "Seed completed: user={}, player_id={}, village_id={}, pos=({}, {})",
                summary.email,
                summary.player_id,
                summary.village_id,
                summary.position_x,
                summary.position_y
            );
            Ok(())
        }
        Err(e) => {
            uow.rollback().await?;
            Err(e)
        }
    }
}

fn parse_seed_path() -> PathBuf {
    let mut args = std::env::args().skip(1);
    match args.next() {
        None => PathBuf::from(DEFAULT_SEED_PATH),
        Some(flag) if flag == "--file" => args
            .next()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SEED_PATH)),
        Some(path) => PathBuf::from(path),
    }
}

fn load_seed_file(path: &PathBuf) -> Result<SeedFile, ApplicationError> {
    let file = File::open(path).map_err(|e| {
        ApplicationError::Unknown(format!("failed to open seed file {}: {e}", path.display()))
    })?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| {
        ApplicationError::Unknown(format!("failed to parse seed file {}: {e}", path.display()))
    })
}

struct SeedSummary {
    email: String,
    player_id: Uuid,
    village_id: u32,
    position_x: i32,
    position_y: i32,
}

async fn seed_game(
    uow: &Box<dyn parabellum_app::uow::UnitOfWork<'_> + '_>,
    seed: &SeedFile,
    config: &Arc<Config>,
) -> Result<SeedSummary, ApplicationError> {
    let password_hash = hash_password(&seed.user.password).map_err(ApplicationError::App)?;
    uow.users()
        .save(seed.user.email.clone(), password_hash)
        .await?;
    let user = uow.users().get_by_email(&seed.user.email).await?;

    let player = Player {
        id: Uuid::new_v4(),
        username: seed.user.username.clone(),
        tribe: seed.user.tribe.clone(),
        user_id: user.id,
        culture_points: 0,
    };
    uow.players().save(&player).await?;

    let valley = uow
        .map()
        .find_unoccupied_valley(&MapQuadrant::from(seed.user.quadrant))
        .await?;
    let village_name = seed
        .village
        .as_ref()
        .and_then(|v| v.name.as_ref())
        .cloned()
        .unwrap_or_else(|| format!("{}'s Village", seed.user.username));
    let mut village = Village::new(
        village_name,
        &valley,
        &player,
        true,
        config.world_size as i32,
        config.speed,
    );

    for slot_id in 1..=18 {
        village.set_building_level_at_slot(
            slot_id,
            seed.resource_fields_target_level,
            config.speed,
        )?;
    }

    for building in &seed.buildings {
        let target_level = match building.level {
            Some(level) => level,
            None => get_building_data(&building.name)?.rules.max_level,
        };

        if let Some(existing) = village.get_building_by_slot_id(building.slot_id) {
            if existing.building.name != building.name {
                return Err(ApplicationError::Unknown(format!(
                    "slot {} has {:?}, expected {:?}",
                    building.slot_id, existing.building.name, building.name
                )));
            }
            village.set_building_level_at_slot(building.slot_id, target_level, config.speed)?;
        } else {
            let built = Building::new(building.name.clone(), config.speed)
                .at_level(target_level, config.speed)?;
            village.add_building_at_slot(built, building.slot_id)?;
        }
    }

    uow.villages().save(&village).await?;
    uow.players().update_culture_points(player.id).await?;

    Ok(SeedSummary {
        email: user.email,
        player_id: player.id,
        village_id: village.id,
        position_x: village.position.x,
        position_y: village.position.y,
    })
}

async fn setup_world_map(pool: &PgPool, config: &Config) -> Result<(), ApplicationError> {
    match bootstrap_world_map(pool, config.world_size).await {
        Ok(true) => println!("World map bootstrapped."),
        Ok(false) => println!("World map already present."),
        Err(e) => {
            return Err(ApplicationError::Unknown(format!(
                "world map bootstrap failed: {e}"
            )));
        }
    }
    Ok(())
}
