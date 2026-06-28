use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Transaction, types::Json};
use uuid::Uuid;

use parabellum_app::{map::MapReadPort, read_models::MapRegionTile};
use parabellum_game::models::map::{
    MapField, MapFieldTopology, MapQuadrant, Valley, generate_new_map,
};
use parabellum_types::{
    errors::{
        ApplicationError,
        DbError::{self},
    },
    map::{Position, ValleyTopology},
};

use crate::ProjectionDb;
use crate::persistence::models as db_models;

#[derive(Clone)]
pub struct PostgresMapRepository {
    pool: PgPool,
}

impl PostgresMapRepository {
    pub fn new(db: ProjectionDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn find_unoccupied_valley_for_update(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        quadrant: &MapQuadrant,
    ) -> Result<Valley, ApplicationError> {
        let query = random_unoccupied_valley_for_update_query(quadrant);
        let random_unoccupied_field: db_models::MapField = sqlx::query_as(query)
            .fetch_one(&mut **tx)
            .await
            .map_err(|_| ApplicationError::Db(DbError::WorldMapNotInitialized))?;
        map_field_to_valley(random_unoccupied_field)
    }

    /// Stores village occupancy for a map field inside an existing transaction.
    pub async fn set_occupancy_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        field_id: u32,
        village_id: Option<u32>,
        player_id: Option<Uuid>,
    ) -> Result<(), ApplicationError> {
        set_occupancy_query(field_id, village_id, player_id)
            .build()
            .execute(&mut **tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl MapReadPort for PostgresMapRepository {
    async fn find_unoccupied_valley(
        &self,
        quadrant: &MapQuadrant,
    ) -> Result<Valley, ApplicationError> {
        let query = random_unoccupied_valley_query(quadrant);

        let random_unoccupied_field: db_models::MapField = sqlx::query_as(query)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| ApplicationError::Db(DbError::WorldMapNotInitialized))?;

        map_field_to_valley(random_unoccupied_field)
    }

    async fn get_field_by_id(&self, id: i32) -> Result<MapField, ApplicationError> {
        let field = sqlx::query_as::<_, db_models::MapField>(
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ApplicationError::Db(DbError::MapFieldNotFound(id as u32)))?;

        Ok(field.into())
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

        let records = sqlx::query_as::<_, DbMapFieldWithOwner>(
            r#"
            SELECT
                mf.id,
                mf.village_id,
                mf.player_id,
                mf.position,
                mf.topology,
                rv.village_name AS village_name,
                rv.population AS village_population,
                rv.is_capital AS is_capital,
                p.username AS player_name,
                p.tribe as tribe
            FROM rm_map_fields AS mf
            LEFT JOIN rm_village AS rv
                ON rv.village_id = mf.village_id
            LEFT JOIN players AS p
                ON p.id = mf.player_id
            WHERE mf.id = ANY($1)
            ORDER BY array_position($1, mf.id)
            "#,
        )
        .bind(&tile_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let fields = records
            .into_iter()
            .map(|record| {
                let db_field = db_models::MapField {
                    id: record.id,
                    village_id: record.village_id,
                    player_id: record.player_id,
                    position: record.position,
                    topology: record.topology,
                };
                MapRegionTile {
                    field: MapField::from(db_field),
                    village_name: record.village_name,
                    village_population: record.village_population,
                    is_capital: record.is_capital,
                    player_name: record.player_name,
                    tribe: record.tribe.map(|t| t.into()),
                }
            })
            .collect();

        Ok(fields)
    }

    async fn get_region_tile_by_field_id(
        &self,
        field_id: i32,
    ) -> Result<Option<MapRegionTile>, ApplicationError> {
        let record = sqlx::query_as::<_, DbMapFieldWithOwner>(
            r#"
            SELECT
                mf.id,
                mf.village_id,
                mf.player_id,
                mf.position,
                mf.topology,
                rv.village_name AS village_name,
                rv.population AS village_population,
                rv.is_capital AS is_capital,
                p.username AS player_name,
                p.tribe as tribe
            FROM rm_map_fields AS mf
            LEFT JOIN rm_village AS rv
                ON rv.village_id = mf.village_id
            LEFT JOIN players AS p
                ON p.id = mf.player_id
            WHERE mf.id = $1
            "#,
        )
        .bind(field_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(record.map(|record| {
            let db_field = db_models::MapField {
                id: record.id,
                village_id: record.village_id,
                player_id: record.player_id,
                position: record.position,
                topology: record.topology,
            };
            MapRegionTile {
                field: MapField::from(db_field),
                village_name: record.village_name,
                village_population: record.village_population,
                is_capital: record.is_capital,
                player_name: record.player_name,
                tribe: record.tribe.map(|t| t.into()),
            }
        }))
    }

    async fn is_unoccupied_valley(&self, field_id: i32) -> Result<bool, ApplicationError> {
        let is_unoccupied_valley: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM rm_map_fields
                WHERE id = $1
                  AND village_id IS NULL
                  AND player_id IS NULL
                  AND topology ? 'Valley'
            )
            "#,
        )
        .bind(field_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(is_unoccupied_valley)
    }

    async fn get_foundation_target_topology(
        &self,
        field_id: u32,
        player_id: Uuid,
    ) -> Result<Option<ValleyTopology>, ApplicationError> {
        let (field_exists, topology_json): (bool, Option<Value>) = sqlx::query_as(
            r#"
            SELECT
              EXISTS(SELECT 1 FROM rm_map_fields WHERE id = $1) AS field_exists,
              (
                SELECT topology
                FROM rm_map_fields
                WHERE id = $1
                  AND village_id IS NULL
                  AND (player_id IS NULL OR player_id = $2)
                  AND topology ? 'Valley'
              ) AS topology
            "#,
        )
        .bind(field_id as i32)
        .bind(player_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        if !field_exists {
            return Err(ApplicationError::Db(DbError::MapFieldNotFound(field_id)));
        }

        let Some(topology_json) = topology_json else {
            return Ok(None);
        };
        let topology = serde_json::from_value::<MapFieldTopology>(topology_json)
            .ok()
            .and_then(|topology| match topology {
                MapFieldTopology::Valley(valley) => Some(valley),
                _ => None,
            });

        Ok(topology)
    }
}

pub(crate) fn random_unoccupied_valley_query(quadrant: &MapQuadrant) -> &'static str {
    match quadrant {
        MapQuadrant::NorthEast => {
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND player_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int > 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1"
        }
        MapQuadrant::SouthEast => {
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND player_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int < 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1"
        }
        MapQuadrant::SouthWest => {
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND player_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int < 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1"
        }
        MapQuadrant::NorthWest => {
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND player_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int > 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1"
        }
    }
}

pub(crate) fn random_unoccupied_valley_for_update_query(quadrant: &MapQuadrant) -> &'static str {
    match quadrant {
        MapQuadrant::NorthEast => {
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND player_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int > 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
        }
        MapQuadrant::SouthEast => {
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND player_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int < 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
        }
        MapQuadrant::SouthWest => {
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND player_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int < 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
        }
        MapQuadrant::NorthWest => {
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND player_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int > 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
        }
    }
}

pub(crate) fn random_unoccupied_4446_valley_for_update_query(
    quadrant: &MapQuadrant,
) -> &'static str {
    match quadrant {
        MapQuadrant::NorthEast => {
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND player_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int > 0 AND topology = '{\"Valley\":[4,4,4,6]}'::jsonb ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
        }
        MapQuadrant::SouthEast => {
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND player_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int < 0 AND topology = '{\"Valley\":[4,4,4,6]}'::jsonb ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
        }
        MapQuadrant::SouthWest => {
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND player_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int < 0 AND topology = '{\"Valley\":[4,4,4,6]}'::jsonb ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
        }
        MapQuadrant::NorthWest => {
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND player_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int > 0 AND topology = '{\"Valley\":[4,4,4,6]}'::jsonb ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
        }
    }
}

fn set_occupancy_query(
    field_id: u32,
    village_id: Option<u32>,
    player_id: Option<Uuid>,
) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new("UPDATE rm_map_fields SET village_id = ");
    query.push_bind(village_id.map(|id| id as i32));
    query.push(", player_id = ");
    query.push_bind(player_id);
    query.push(", updated_at = NOW() WHERE id = ");
    query.push_bind(field_id as i32);
    query
}

fn map_field_to_valley(field: db_models::MapField) -> Result<Valley, ApplicationError> {
    let game_map_field: MapField = field.into();
    Valley::try_from(game_map_field.clone())
        .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(game_map_field.id)))
}

#[derive(Debug, FromRow)]
struct DbMapFieldWithOwner {
    id: i32,
    village_id: Option<i32>,
    player_id: Option<Uuid>,
    position: Value,
    topology: Value,
    village_name: Option<String>,
    village_population: Option<i32>,
    is_capital: Option<bool>,
    player_name: Option<String>,
    tribe: Option<db_models::Tribe>,
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

/// Populates the World Map.
pub async fn bootstrap_world_map(pool: &PgPool, world_size: i16) -> Result<bool, ApplicationError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rm_map_fields")
        .fetch_one(pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

    if count == 0 {
        tracing::info!("Generating new World Map");
        let map_fields = generate_new_map(world_size as i32);

        const BATCH_SIZE: usize = 10_000;
        for chunk in map_fields.chunks(BATCH_SIZE) {
            let mut tx = pool.begin().await.map_err(DbError::Database)?;
            let mut query_builder = QueryBuilder::new(
                "INSERT INTO rm_map_fields (id, village_id, player_id, position, topology) ",
            );

            query_builder.push_values(chunk.iter(), |mut q, field| {
                q.push_bind(field.id as i32)
                    .push_bind(field.village_id.map(|id| id as i32))
                    .push_bind(field.player_id)
                    .push_bind(Json(&field.position))
                    .push_bind(Json(&field.topology));
            });

            let query = query_builder.build();
            query
                .execute(&mut *tx.as_mut())
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

            tx.commit()
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        }
    }

    Ok(count == 0)
}
