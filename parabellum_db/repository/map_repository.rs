use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;

use parabellum_app::repository::MapRepository;
use parabellum_core::{ApplicationError, DbError};
use parabellum_game::models::map::{MapField, MapQuadrant, Valley};

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
            .map_err(|_| ApplicationError::Db(DbError::MapFieldNotFound(0)))?; // use id zero because of randomization

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
