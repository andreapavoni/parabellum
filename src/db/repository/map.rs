use anyhow::Result;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::game::models::map::{MapQuadrant, Valley};
use crate::repository::MapRepository;
use crate::{db::models as db_models, game::models::map::MapField};

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
    async fn find_unoccupied_valley(&self, quadrant: &MapQuadrant) -> Result<Valley> {
        let mut tx_guard = self.tx.lock().await;
        let query = match quadrant {
          MapQuadrant::NorthEast => "SELECT * FROM map_fields WHERE village_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int > 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1",
          MapQuadrant::SouthEast => "SELECT * FROM map_fields WHERE village_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int < 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1",
          MapQuadrant::SouthWest => "SELECT * FROM map_fields WHERE village_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int < 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1",
          MapQuadrant::NorthWest => "SELECT * FROM map_fields WHERE village_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int > 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1",
      };

        let random_unoccupied_field: db_models::MapField = sqlx::query_as(query)
            .fetch_one(&mut *tx_guard.as_mut())
            .await?;

        let game_map_field: MapField = random_unoccupied_field.into();
        let valley = Valley::try_from(game_map_field)?;

        Ok(valley)
    }

    async fn get_field_by_id(&self, id: i32) -> Result<Option<MapField>> {
        let mut tx_guard = self.tx.lock().await;
        let field = sqlx::query_as!(
            db_models::MapField,
            "SELECT * FROM map_fields WHERE id = $1",
            id
        )
        .fetch_optional(&mut *tx_guard.as_mut())
        .await?;

        Ok(field.map(Into::into))
    }
}
