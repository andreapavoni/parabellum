use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::{MapRegionTile, MapRepository};
use parabellum_game::models::map::{MapField, MapQuadrant, Valley};
use parabellum_types::{
    errors::{
        ApplicationError,
        DbError::{self},
    },
    map::Position,
};

use crate::{
    models as db_models,
    toasty_models::{map_field::MapFieldDbRow, player::PlayerRecord, village::VillageDbRow},
};

pub struct ToastyMapRepository<'a> {
    tx: Arc<Mutex<toasty::Transaction<'a>>>,
}

impl<'a> ToastyMapRepository<'a> {
    pub fn new(tx: Arc<Mutex<toasty::Transaction<'a>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> MapRepository for ToastyMapRepository<'a> {
    async fn find_unoccupied_valley(
        &self,
        quadrant: &MapQuadrant,
    ) -> Result<Valley, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let rows = MapFieldDbRow::all()
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        let mut candidates = Vec::new();
        for row in rows {
            if row.village_id.is_some() {
                continue;
            }
            if row.topology != serde_json::json!({ "Valley": [4, 4, 4, 6] }) {
                continue;
            }

            let position: Position = serde_json::from_value(row.position.clone()).map_err(|e| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "invalid map field position payload for {}: {}",
                    row.id, e
                )))
            })?;

            let in_quadrant = match quadrant {
                MapQuadrant::NorthEast => position.x > 0 && position.y > 0,
                MapQuadrant::SouthEast => position.x > 0 && position.y < 0,
                MapQuadrant::SouthWest => position.x < 0 && position.y < 0,
                MapQuadrant::NorthWest => position.x < 0 && position.y > 0,
            };

            if in_quadrant {
                candidates.push(row);
            }
        }

        let row = candidates
            .into_iter()
            .next()
            .ok_or_else(|| ApplicationError::Db(DbError::WorldMapNotInitialized))?;
        let game_map_field: MapField = db_models::MapField::from(row).into();
        Valley::try_from(game_map_field.clone())
            .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(game_map_field.id)))
    }

    async fn get_field_by_id(&self, id: i32) -> Result<MapField, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let row = MapFieldDbRow::get_by_id(&mut *tx_guard, id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::MapFieldNotFound(id as u32)))?;
        Ok(db_models::MapField::from(row).into())
    }

    async fn get_region(
        &self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        world_size: i32,
    ) -> Result<Vec<MapRegionTile>, ApplicationError> {
        let tile_ids = build_region_ids(center_x, center_y, radius, world_size);
        if tile_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx_guard = self.tx.lock().await;
        let fields = MapFieldDbRow::all()
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        let mut fields_by_id = HashMap::new();
        for field in fields {
            fields_by_id.insert(field.id, field);
        }

        let tile_set: std::collections::HashSet<i32> = tile_ids.iter().copied().collect();
        let villages = VillageDbRow::all()
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        let villages_by_id: HashMap<i32, VillageDbRow> = villages
            .into_iter()
            .filter(|village| tile_set.contains(&village.id))
            .map(|village| (village.id, village))
            .collect();

        let mut players_cache: HashMap<Uuid, PlayerRecord> = HashMap::new();
        let mut tiles = Vec::with_capacity(tile_ids.len());

        for tile_id in tile_ids {
            let Some(field_row) = fields_by_id.remove(&tile_id) else {
                continue;
            };
            let fallback_village = villages_by_id.get(&field_row.id);
            let owner_player_id = field_row
                .player_id
                .or_else(|| fallback_village.map(|village| village.player_id));

            let owner_player = if let Some(player_id) = owner_player_id {
                if !players_cache.contains_key(&player_id) {
                    if let Ok(player) = PlayerRecord::get_by_id(&mut *tx_guard, player_id).await {
                        players_cache.insert(player_id, player);
                    }
                }
                players_cache.get(&player_id)
            } else {
                None
            };

            let db_field = db_models::MapField {
                id: field_row.id,
                village_id: field_row.village_id.or_else(|| fallback_village.map(|v| v.id)),
                player_id: owner_player_id,
                position: field_row.position,
                topology: field_row.topology,
            };

            tiles.push(MapRegionTile {
                field: MapField::from(db_field),
                village_name: fallback_village.map(|village| village.name.clone()),
                village_population: fallback_village.map(|village| village.population),
                player_name: owner_player.map(|player| player.username.clone()),
                tribe: owner_player.map(|player| player.tribe.into()),
            });
        }

        Ok(tiles)
    }
}

fn build_region_ids(center_x: i32, center_y: i32, radius: i32, world_size: i32) -> Vec<i32> {
    let diameter = (radius * 2 + 1).max(0) as usize;
    let mut ids = Vec::with_capacity(diameter * diameter);

    for y in ((center_y - radius)..=(center_y + radius)).rev() {
        let wrapped_y = wrap_coordinate(y, world_size);
        for x in center_x - radius..=center_x + radius {
            let wrapped_x = wrap_coordinate(x, world_size);
            let position = Position {
                x: wrapped_x,
                y: wrapped_y,
            };
            ids.push(position.to_id(world_size) as i32);
        }
    }

    ids
}

fn wrap_coordinate(value: i32, world_size: i32) -> i32 {
    if world_size <= 0 {
        return value;
    }
    let span = world_size * 2 + 1;
    let mut normalized = (value + world_size) % span;
    if normalized < 0 {
        normalized += span;
    }
    normalized - world_size
}

fn map_toasty_error(err: toasty::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Transaction(err.to_string()))
}
