use sqlx::{PgPool, Postgres, QueryBuilder, Transaction, types::Json};
use std::sync::Arc;
use tokio::sync::Mutex;

use parabellum_app::repository::MapRepository;
use parabellum_game::models::map::{MapField, MapQuadrant, Valley, generate_new_map};
use parabellum_types::errors::{
    ApplicationError,
    DbError::{self},
};

use crate::models as db_models;

#[derive(Clone)]
pub struct PostgresMapRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresMapRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> MapRepository for PostgresMapRepository<'a> {
    async fn find_unoccupied_valley(
        &self,
        quadrant: &MapQuadrant,
    ) -> Result<Valley, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let query = match quadrant {
            MapQuadrant::NorthEast => {
                "SELECT * FROM map_fields WHERE village_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int > 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1"
            }
            MapQuadrant::SouthEast => {
                "SELECT * FROM map_fields WHERE village_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int < 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1"
            }
            MapQuadrant::SouthWest => {
                "SELECT * FROM map_fields WHERE village_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int < 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1"
            }
            MapQuadrant::NorthWest => {
                "SELECT * FROM map_fields WHERE village_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int > 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1"
            }
        };

        let random_unoccupied_field: db_models::MapField = sqlx::query_as(query)
            .fetch_one(&mut *tx_guard.as_mut())
            .await
            .map_err(|_| ApplicationError::Db(DbError::WorldMapNotInitialized))?;

        let game_map_field: MapField = random_unoccupied_field.into();
        let valley = Valley::try_from(game_map_field.clone())
            .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(game_map_field.id)))?;

        Ok(valley)
    }

    async fn get_field_by_id(&self, id: i32) -> Result<MapField, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let field = sqlx::query_as!(
            db_models::MapField,
            "SELECT * FROM map_fields WHERE id = $1",
            id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::MapFieldNotFound(id as u32)))?;

        Ok(field.into())
    }
}

/// Populates the World Map.
pub async fn bootstrap_world_map(pool: &PgPool, world_size: i16) -> Result<bool, ApplicationError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM map_fields")
        .fetch_one(pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

    if count > 0 {
        return Ok(false);
    }

    tracing::info!("Generating new World Map");
    let map_fields = generate_new_map(world_size as i32);

    const BATCH_SIZE: usize = 10_000;
    for chunk in map_fields.chunks(BATCH_SIZE) {
        let mut tx = pool.begin().await.map_err(DbError::Database)?;
        let mut query_builder = QueryBuilder::new(
            "INSERT INTO map_fields (id, village_id, player_id, position, topology) ",
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

    Ok(true)
}
