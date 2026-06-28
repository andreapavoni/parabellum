//! Postgres marketplace projection repository.
//!
//! Marketplace offers are projection rows. Merchant movement views are derived
//! from active scheduled merchant actions and use the shared scheduled-action
//! filter/query path.

mod queries;
mod rows;
mod writes;

use chrono::{DateTime, Utc};
use parabellum_app::villages::models::{MarketplaceOfferModel, MarketplaceOfferStatus};
use parabellum_app::villages::projection_repositories::{
    MarketplaceOfferListFilter, MarketplaceRepository,
};
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::PgPool;
use uuid::Uuid;

use self::rows::DbMarketplaceOfferRow;
use crate::ProjectionDb;

#[derive(Debug, Clone)]
pub struct PostgresMarketplaceRepository {
    pool: PgPool,
}

impl PostgresMarketplaceRepository {
    pub fn new(db: ProjectionDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn list_open_by_owner_village_id(
        &self,
        village_id: u32,
    ) -> Result<Vec<MarketplaceOfferModel>, ApplicationError> {
        self.list_offers_by_filter(
            MarketplaceOfferListFilter::new()
                .owner_village(village_id)
                .open(),
        )
        .await
    }

    pub async fn list_open_excluding_owner_village_id(
        &self,
        village_id: u32,
    ) -> Result<Vec<MarketplaceOfferModel>, ApplicationError> {
        self.list_offers_by_filter(
            MarketplaceOfferListFilter::new()
                .excluding_owner_village(village_id)
                .open(),
        )
        .await
    }

    pub(super) fn pool(&self) -> &PgPool {
        &self.pool
    }

    async fn list_offers_by_filter(
        &self,
        filter: MarketplaceOfferListFilter,
    ) -> Result<Vec<MarketplaceOfferModel>, ApplicationError> {
        let rows: Vec<DbMarketplaceOfferRow> = queries::marketplace_offer_query(filter)
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait::async_trait]
impl MarketplaceRepository for PostgresMarketplaceRepository {
    async fn upsert(&self, offer: &MarketplaceOfferModel) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        self.upsert_in_tx(&mut tx, offer).await?;
        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn get_by_offer_id(
        &self,
        offer_id: Uuid,
    ) -> Result<MarketplaceOfferModel, ApplicationError> {
        let row: DbMarketplaceOfferRow = queries::marketplace_offer_by_id_query(offer_id)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(row.into())
    }

    async fn set_status(
        &self,
        offer_id: Uuid,
        status: MarketplaceOfferStatus,
        accepted_by_player_id: Option<Uuid>,
        accepted_by_village_id: Option<u32>,
        at: DateTime<Utc>,
    ) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        self.set_status_in_tx(
            &mut tx,
            offer_id,
            status,
            accepted_by_player_id,
            accepted_by_village_id,
            at,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn list_offers(
        &self,
        filter: MarketplaceOfferListFilter,
    ) -> Result<Vec<MarketplaceOfferModel>, ApplicationError> {
        self.list_offers_by_filter(filter).await
    }

    async fn claim_open_for_accept(
        &self,
        offer_id: Uuid,
        accepted_by_player_id: Uuid,
        accepted_by_village_id: u32,
        at: DateTime<Utc>,
    ) -> Result<Option<MarketplaceOfferModel>, ApplicationError> {
        let row = self
            .claim_open_for_accept_row(offer_id, accepted_by_player_id, accepted_by_village_id, at)
            .await?;
        Ok(row.map(Into::into))
    }
}
