use sqlx::{Postgres, Transaction, types::Json};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::MarketplaceRepository;
use parabellum_core::{ApplicationError, DbError, Result};
use parabellum_game::models::marketplace::MarketplaceOffer;

use crate::models::{self as db_models};

#[derive(Clone)]
pub struct PostgresMarketplaceRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresMarketplaceRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> MarketplaceRepository for PostgresMarketplaceRepository<'a> {
    async fn create(&self, offer: &MarketplaceOffer) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO marketplace_offers (id, player_id, village_id, offer_resources, seek_resources, merchants_required, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            offer.id,
            offer.player_id,
            offer.village_id as i32,
            Json(&offer.offer_resources) as _,
            Json(&offer.seek_resources) as _,
            offer.merchants_required as i16,
            offer.created_at
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_by_id(&self, offer_id: Uuid) -> Result<MarketplaceOffer, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let db_offer = sqlx::query_as!(
            db_models::MarketplaceOffer,
            "SELECT * FROM marketplace_offers WHERE id = $1",
            offer_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(db_offer.into())
    }

    async fn list_by_village(
        &self,
        village_id: u32,
    ) -> Result<Vec<MarketplaceOffer>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let db_offers = sqlx::query_as!(
            db_models::MarketplaceOffer,
            "SELECT * FROM marketplace_offers WHERE village_id = $1 ORDER BY created_at DESC",
            village_id as i32
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(db_offers.into_iter().map(|o| o.into()).collect())
    }

    async fn list_all(&self) -> Result<Vec<MarketplaceOffer>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let db_offers = sqlx::query_as!(
            db_models::MarketplaceOffer,
            "SELECT * FROM marketplace_offers ORDER BY created_at DESC"
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(db_offers.into_iter().map(|o| o.into()).collect())
    }

    async fn delete(&self, offer_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        sqlx::query!("DELETE FROM marketplace_offers WHERE id = $1", offer_id)
            .execute(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }
}
