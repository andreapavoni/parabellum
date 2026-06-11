use std::fs;
use std::path::Path;

use parabellum_app::config::Config;
use parabellum_app::ports::identity::{IdentityPort, InitialVillageSetup, RegisterPlayerRequest};
use parabellum_app::villages::{
    FoundVillage, SetVillageResources, VillageArmyContext, hydrate_village,
};
use parabellum_game::models::map::MapFieldTopology;
use parabellum_game::models::{buildings::Building, village::VillageBuilding};
use parabellum_types::army::UnitName;
use parabellum_types::buildings::BuildingName;
use parabellum_types::common::ResourceGroup;
use parabellum_types::errors::{ApplicationError, DbError};
use parabellum_types::map::{Position, ValleyTopology};
use parabellum_types::tribe::Tribe;
use serde::Deserialize;
use sqlx::types::Json;
use uuid::Uuid;

use crate::bootstrap_world_map;
use crate::es::{PostgresArmyRepository, VillageEsService};
use crate::identity::IdentityService;
use crate::map::PostgresMapRepository;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedFile {
    pub players: Vec<SeedPlayer>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedPlayer {
    pub username: String,
    pub email: Option<String>,
    pub password: Option<String>,
    #[serde(default = "default_tribe")]
    pub tribe: Tribe,
    pub villages: Vec<SeedVillage>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedVillage {
    pub template: Option<String>,
    pub name: Option<String>,
    #[serde(default = "default_resource_fields_target_level")]
    pub resource_fields_target_level: u8,
    #[serde(default)]
    pub buildings: Vec<SeedBuilding>,
    #[serde(default)]
    pub academy_researches: Vec<UnitName>,
    #[serde(default)]
    pub starting_army: Vec<SeedUnitAmount>,
    pub position: Option<Position>,
    pub resources: Option<ResourceGroup>,
    pub speed: Option<i8>,
    #[serde(default = "default_quadrant")]
    pub quadrant: SeedQuadrant,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeedBuilding {
    #[serde(rename = "slotId")]
    pub slot_id: u8,
    pub name: BuildingName,
    #[serde(default = "default_building_level")]
    pub level: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeedUnitAmount {
    pub unit: UnitName,
    pub quantity: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub enum SeedQuadrant {
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl SeedQuadrant {
    fn as_domain(&self) -> parabellum_game::models::map::MapQuadrant {
        match self {
            Self::NorthEast => parabellum_game::models::map::MapQuadrant::NorthEast,
            Self::SouthEast => parabellum_game::models::map::MapQuadrant::SouthEast,
            Self::SouthWest => parabellum_game::models::map::MapQuadrant::SouthWest,
            Self::NorthWest => parabellum_game::models::map::MapQuadrant::NorthWest,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SeedRunResult {
    pub username: String,
    pub email: String,
    pub password: String,
    pub village_name: String,
    pub village_id: u32,
    pub village_position: Position,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedVillageTemplate {
    resource_fields_target_level: Option<u8>,
    buildings: Option<Vec<SeedBuilding>>,
    academy_researches: Option<Vec<UnitName>>,
    starting_army: Option<Vec<SeedUnitAmount>>,
    resources: Option<ResourceGroup>,
    speed: Option<i8>,
    quadrant: Option<SeedQuadrant>,
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

pub fn parse_seed_file(raw: &str) -> Result<SeedFile, ApplicationError> {
    let parsed: SeedFile = serde_json::from_str(raw)
        .map_err(|e| ApplicationError::Unknown(format!("invalid seed JSON: {e}")))?;
    if parsed.players.is_empty() {
        return Err(ApplicationError::Unknown(
            "invalid seed JSON: players cannot be empty".to_string(),
        ));
    }
    for player in &parsed.players {
        if player.villages.is_empty() {
            return Err(ApplicationError::Unknown(format!(
                "invalid seed JSON: player '{}' must define at least one village",
                player.username
            )));
        }
    }
    Ok(parsed)
}

pub async fn run_seed(
    pool: &sqlx::PgPool,
    config: &Config,
    seed: SeedFile,
    seed_file_path: &Path,
) -> Result<Vec<SeedRunResult>, ApplicationError> {
    ensure_migrations_and_map(pool, config).await?;
    let map_repository = PostgresMapRepository::new(pool.clone());
    let service = VillageEsService::new(pool.clone());
    let mut out = Vec::new();

    for player in seed.players {
        let first_village = resolve_village_from_template(&player.villages[0], seed_file_path)?;
        if first_village.position.is_some() {
            return Err(ApplicationError::Unknown(format!(
                "player '{}' first village must not define position; signup flow always uses random valley by quadrant",
                player.username
            )));
        }

        let email = player
            .email
            .clone()
            .unwrap_or_else(|| format!("{}@example.com", player.username));
        let password = player
            .password
            .clone()
            .unwrap_or_else(|| player.username.clone());
        let player_id = Uuid::new_v4();
        let identity = IdentityService::new(pool.clone(), std::sync::Arc::new(config.clone()));

        identity
            .register_player(RegisterPlayerRequest {
                player_id,
                username: player.username.clone(),
                email: email.clone(),
                password: password.clone(),
                tribe: player.tribe.clone(),
                quadrant: first_village.quadrant.as_domain(),
                initial_village: Some(InitialVillageSetup {
                    village_name: first_village.name.clone(),
                    resource_fields_target_level: first_village.resource_fields_target_level,
                    buildings: village_buildings(
                        first_village.speed.unwrap_or(config.speed),
                        &first_village,
                    )?
                    .into_iter()
                    .filter(|b| b.slot_id >= 19)
                    .collect(),
                    resources: first_village.resources.clone(),
                    speed: first_village.speed,
                }),
            })
            .await?;

        let created_villages = service
            .list_villages_by_player_id(player_id)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        let first = created_villages
            .iter()
            .find(|v| v.parent_village_id.is_none())
            .or_else(|| created_villages.first())
            .ok_or_else(|| {
                ApplicationError::Unknown(format!(
                    "cannot locate initial village for player '{}'",
                    player.username
                ))
            })?;

        out.push(SeedRunResult {
            username: player.username.clone(),
            email: email.clone(),
            password: password.clone(),
            village_name: first.village_name.clone(),
            village_id: first.village_id,
            village_position: first.position.clone(),
        });
        apply_village_seed_state(pool, &service, player_id, first.village_id, &first_village)
            .await?;

        let mut previous_village_id = Some(first.village_id);
        for (village_idx, village) in player.villages.iter().enumerate().skip(1) {
            let village = resolve_village_from_template(village, seed_file_path)?;
            let (village_id, village_position, soft_reserved) = if let Some(pos) =
                village.position.clone()
            {
                let village_id = pos.to_id(config.world_size as i32);
                ensure_map_field_is_free(pool, village_id).await?;
                reserve_map_field_for_player(pool, village_id, player_id).await?;
                (village_id, pos, true)
            } else {
                claim_random_unoccupied_valley(pool, &map_repository, &village.quadrant, player_id)
                    .await?
            };

            let speed = village.speed.unwrap_or(config.speed);
            let topology = load_valley_topology(pool, village_id).await?;

            let mut resource_buildings = topology_resource_buildings(
                &topology,
                speed,
                village.resource_fields_target_level,
            )?;
            let mut extra_buildings = village_buildings(speed, &village)?
                .into_iter()
                .filter(|b| b.slot_id >= 19)
                .collect::<Vec<_>>();
            resource_buildings.append(&mut extra_buildings);
            normalize_buildings_by_slot(&mut resource_buildings);

            let village_name = village.name.clone().unwrap_or_else(|| {
                if village_idx == 0 {
                    format!("{}'s Village", player.username)
                } else {
                    format!("{}'s Village {}", player.username, village_idx + 1)
                }
            });

            let found_result = service
                .found_village(
                    village_id,
                    &FoundVillage {
                        village_name: village_name.clone(),
                        position: village_position.clone(),
                        tribe: player.tribe.clone(),
                        player_id,
                        parent_village_id: previous_village_id,
                        buildings: resource_buildings,
                    },
                )
                .await
                .map_err(|e| ApplicationError::Infrastructure(e.to_string()));
            if let Err(err) = found_result {
                if soft_reserved {
                    release_map_field_player_reservation(pool, village_id, player_id).await?;
                }
                return Err(err);
            }

            service
                .set_village_resources(
                    village_id,
                    &SetVillageResources {
                        player_id,
                        resources: village
                            .resources
                            .clone()
                            .unwrap_or_else(|| ResourceGroup::new(80_000, 80_000, 80_000, 80_000)),
                    },
                )
                .await
                .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

            out.push(SeedRunResult {
                username: player.username.clone(),
                email: email.clone(),
                password: password.clone(),
                village_name,
                village_id,
                village_position,
            });
            apply_village_seed_state(pool, &service, player_id, village_id, &village).await?;
            previous_village_id = Some(village_id);
        }
    }

    Ok(out)
}

fn village_buildings(
    config_speed: i8,
    seed: &SeedVillage,
) -> Result<Vec<VillageBuilding>, ApplicationError> {
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

fn resolve_village_from_template(
    village: &SeedVillage,
    seed_file_path: &Path,
) -> Result<SeedVillage, ApplicationError> {
    let Some(template_name) = &village.template else {
        return Ok(village.clone());
    };
    let Some(seed_dir) = seed_file_path.parent() else {
        return Err(ApplicationError::Unknown(
            "cannot resolve seed template directory".to_string(),
        ));
    };
    let template_path = seed_dir
        .join("templates")
        .join(format!("{template_name}.json"));
    let raw = fs::read_to_string(&template_path).map_err(|e| {
        ApplicationError::Unknown(format!(
            "cannot read template {}: {e}",
            template_path.display()
        ))
    })?;
    let template: SeedVillageTemplate = serde_json::from_str(&raw).map_err(|e| {
        ApplicationError::Unknown(format!(
            "invalid template JSON {}: {e}",
            template_path.display()
        ))
    })?;

    let mut merged = village.clone();
    if let Some(resource_fields_target_level) = template.resource_fields_target_level {
        merged.resource_fields_target_level = resource_fields_target_level;
    }
    if let Some(resources) = template.resources {
        merged.resources = Some(resources);
    }
    if let Some(speed) = template.speed {
        merged.speed = Some(speed);
    }
    if let Some(quadrant) = template.quadrant {
        merged.quadrant = quadrant;
    }
    if let Some(template_buildings) = template.buildings {
        let mut buildings = template_buildings;
        for b in &village.buildings {
            upsert_building(&mut buildings, b.clone());
        }
        merged.buildings = buildings;
    }
    if let Some(template_academy_researches) = template.academy_researches {
        let mut researches = template_academy_researches;
        for unit in &village.academy_researches {
            if !researches.contains(unit) {
                researches.push(unit.clone());
            }
        }
        merged.academy_researches = researches;
    }
    if let Some(template_starting_army) = template.starting_army {
        let mut starting_army = template_starting_army;
        for unit_amount in &village.starting_army {
            upsert_unit_amount(&mut starting_army, unit_amount.clone());
        }
        merged.starting_army = starting_army;
    }
    Ok(merged)
}

fn upsert_building(buildings: &mut Vec<SeedBuilding>, building: SeedBuilding) {
    if let Some(existing) = buildings.iter_mut().find(|b| b.slot_id == building.slot_id) {
        *existing = building;
        return;
    }
    buildings.push(building);
}

fn upsert_unit_amount(units: &mut Vec<SeedUnitAmount>, unit_amount: SeedUnitAmount) {
    if let Some(existing) = units.iter_mut().find(|u| u.unit == unit_amount.unit) {
        *existing = unit_amount;
        return;
    }
    units.push(unit_amount);
}

async fn apply_village_seed_state(
    pool: &sqlx::PgPool,
    service: &VillageEsService,
    player_id: Uuid,
    village_id: u32,
    seed: &SeedVillage,
) -> Result<(), ApplicationError> {
    if seed.academy_researches.is_empty() && seed.starting_army.is_empty() {
        return Ok(());
    }

    let model = service
        .get_village(village_id)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
    let mut village = hydrate_village(model, VillageArmyContext::default());

    for unit in &seed.academy_researches {
        village
            .research_academy(unit.clone())
            .map_err(ApplicationError::from)?;
    }
    for unit_amount in &seed.starting_army {
        if unit_amount.quantity == 0 {
            continue;
        }
        village
            .add_trained_units_home(unit_amount.unit.clone(), unit_amount.quantity)
            .map_err(ApplicationError::from)?;
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
    if let Some(army) = village.army() {
        PostgresArmyRepository::new(pool.clone())
            .upsert_home_in_tx(&mut tx, army, player_id)
            .await?;
    }
    sqlx::query(
        r#"
        UPDATE rm_village
        SET academy_research = $2,
            production = $3,
            population = $4,
            culture_points_production = $5,
            updated_at = NOW()
        WHERE village_id = $1
        "#,
    )
    .bind(village_id as i32)
    .bind(Json(village.academy_research().clone()))
    .bind(Json(village.production.clone()))
    .bind(village.population as i32)
    .bind(village.culture_points_production as i32)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
    tx.commit()
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
    Ok(())
}

fn normalize_buildings_by_slot(buildings: &mut Vec<VillageBuilding>) {
    let mut normalized = Vec::with_capacity(buildings.len());
    for building in buildings.drain(..) {
        if let Some(existing) = normalized
            .iter_mut()
            .find(|b: &&mut VillageBuilding| b.slot_id == building.slot_id)
        {
            *existing = building;
        } else {
            normalized.push(building);
        }
    }
    *buildings = normalized;
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

async fn ensure_map_field_is_free(
    pool: &sqlx::PgPool,
    village_id: u32,
) -> Result<(), ApplicationError> {
    let occupied: bool = sqlx::query_scalar(
        "SELECT village_id IS NOT NULL OR player_id IS NOT NULL FROM rm_map_fields WHERE id = $1",
    )
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

async fn claim_random_unoccupied_valley(
    pool: &sqlx::PgPool,
    map_repository: &PostgresMapRepository,
    quadrant: &SeedQuadrant,
    player_id: Uuid,
) -> Result<(u32, Position, bool), ApplicationError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

    let random_valley = map_repository
        .find_unoccupied_valley_for_update(&mut tx, &quadrant.as_domain())
        .await?;

    reserve_map_field_for_player_in_tx(&mut tx, random_valley.id, player_id).await?;

    tx.commit()
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

    Ok((random_valley.id, random_valley.position, true))
}

async fn reserve_map_field_for_player(
    pool: &sqlx::PgPool,
    village_id: u32,
    player_id: Uuid,
) -> Result<(), ApplicationError> {
    let updated = sqlx::query(
        "UPDATE rm_map_fields SET player_id = $2 WHERE id = $1 AND village_id IS NULL AND player_id IS NULL",
    )
    .bind(village_id as i32)
    .bind(player_id)
    .execute(pool)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
    .rows_affected();
    if updated == 1 {
        return Ok(());
    }

    Err(ApplicationError::Unknown(format!(
        "cannot reserve selected map field {}",
        village_id
    )))
}

async fn reserve_map_field_for_player_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    village_id: u32,
    player_id: Uuid,
) -> Result<(), ApplicationError> {
    let updated = sqlx::query(
        "UPDATE rm_map_fields SET player_id = $2 WHERE id = $1 AND village_id IS NULL AND player_id IS NULL",
    )
    .bind(village_id as i32)
    .bind(player_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
    .rows_affected();

    if updated == 1 {
        return Ok(());
    }

    Err(ApplicationError::Unknown(format!(
        "cannot reserve selected map field {}",
        village_id
    )))
}

async fn release_map_field_player_reservation(
    pool: &sqlx::PgPool,
    village_id: u32,
    player_id: Uuid,
) -> Result<(), ApplicationError> {
    sqlx::query(
        "UPDATE rm_map_fields SET player_id = NULL WHERE id = $1 AND village_id IS NULL AND player_id = $2",
    )
    .bind(village_id as i32)
    .bind(player_id)
    .execute(pool)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
    Ok(())
}
