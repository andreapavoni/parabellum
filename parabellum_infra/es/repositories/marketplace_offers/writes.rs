//! Write helpers for marketplace offer projections.

use chrono::{DateTime, Utc};
use parabellum_app::villages::models::{MarketplaceOfferModel, MarketplaceOfferStatus};
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{Postgres, QueryBuilder, Transaction, types::Json};
use uuid::Uuid;

use super::{
    PostgresMarketplaceRepository, queries,
    rows::{DbMarketplaceOfferRow, DbMarketplaceOfferStatus},
};

impl PostgresMarketplaceRepository {
    pub async fn upsert_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        offer: &MarketplaceOfferModel,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            INSERT INTO rm_marketplace_offers (
                offer_id, owner_player_id, owner_village_id, offer_resources, seek_resources,
                merchants_reserved, status, accepted_by_player_id, accepted_by_village_id,
                created_at, accepted_at, canceled_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
            )
            ON CONFLICT (offer_id)
            DO UPDATE SET
                owner_player_id = EXCLUDED.owner_player_id,
                owner_village_id = EXCLUDED.owner_village_id,
                offer_resources = EXCLUDED.offer_resources,
                seek_resources = EXCLUDED.seek_resources,
                merchants_reserved = EXCLUDED.merchants_reserved,
                status = EXCLUDED.status,
                accepted_by_player_id = EXCLUDED.accepted_by_player_id,
                accepted_by_village_id = EXCLUDED.accepted_by_village_id,
                created_at = EXCLUDED.created_at,
                accepted_at = EXCLUDED.accepted_at,
                canceled_at = EXCLUDED.canceled_at
            "#,
        )
        .bind(offer.offer_id)
        .bind(offer.owner_player_id)
        .bind(offer.owner_village_id as i32)
        .bind(Json(offer.offer_resources))
        .bind(Json(offer.seek_resources))
        .bind(offer.merchants_reserved as i16)
        .bind(DbMarketplaceOfferStatus::from(offer.status))
        .bind(offer.accepted_by_player_id)
        .bind(offer.accepted_by_village_id.map(|v| v as i32))
        .bind(offer.created_at)
        .bind(offer.accepted_at)
        .bind(offer.canceled_at)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn set_status_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        offer_id: Uuid,
        status: MarketplaceOfferStatus,
        accepted_by_player_id: Option<Uuid>,
        accepted_by_village_id: Option<u32>,
        at: DateTime<Utc>,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_marketplace_offers
            SET status = $2,
                accepted_by_player_id = $3,
                accepted_by_village_id = $4,
                accepted_at = CASE WHEN $2 = 'accepted' THEN $5 ELSE accepted_at END,
                canceled_at = CASE WHEN $2 = 'canceled' THEN $5 ELSE canceled_at END
            WHERE offer_id = $1
            "#,
        )
        .bind(offer_id)
        .bind(DbMarketplaceOfferStatus::from(status))
        .bind(accepted_by_player_id)
        .bind(accepted_by_village_id.map(|v| v as i32))
        .bind(at)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub(super) async fn claim_open_for_accept_row(
        &self,
        offer_id: Uuid,
        accepted_by_player_id: Uuid,
        accepted_by_village_id: u32,
        at: DateTime<Utc>,
    ) -> Result<Option<DbMarketplaceOfferRow>, ApplicationError> {
        let mut query = QueryBuilder::new(
            r#"
            UPDATE rm_marketplace_offers
            SET status = 'accepted'
            "#,
        );
        query.push(", accepted_by_player_id = ");
        query.push_bind(accepted_by_player_id);
        query.push(", accepted_by_village_id = ");
        query.push_bind(accepted_by_village_id as i32);
        query.push(", accepted_at = ");
        query.push_bind(at);
        query.push(" WHERE offer_id = ");
        query.push_bind(offer_id);
        query.push(" AND status = 'open'");
        queries::push_marketplace_offer_returning(&mut query);

        let row = query
            .build_query_as()
            .fetch_optional(self.pool())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(row)
    }
}
