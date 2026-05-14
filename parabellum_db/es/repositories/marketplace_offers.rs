use chrono::{DateTime, Utc};
use parabellum_app::ports::queries::{MerchantMovement, MerchantMovementKind};
use parabellum_app::villages::models::{MarketplaceOfferModel, MarketplaceOfferStatus};
use parabellum_app::villages::repositories::MarketplaceRepository;
use parabellum_types::common::ResourceGroup;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{FromRow, PgPool, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PostgresMarketplaceRepository {
    pool: PgPool,
}

impl PostgresMarketplaceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "rm_marketplace_offer_status", rename_all = "snake_case")]
enum DbMarketplaceOfferStatus {
    Open,
    Accepted,
    Canceled,
}

impl From<MarketplaceOfferStatus> for DbMarketplaceOfferStatus {
    fn from(value: MarketplaceOfferStatus) -> Self {
        match value {
            MarketplaceOfferStatus::Open => Self::Open,
            MarketplaceOfferStatus::Accepted => Self::Accepted,
            MarketplaceOfferStatus::Canceled => Self::Canceled,
        }
    }
}

impl From<DbMarketplaceOfferStatus> for MarketplaceOfferStatus {
    fn from(value: DbMarketplaceOfferStatus) -> Self {
        match value {
            DbMarketplaceOfferStatus::Open => Self::Open,
            DbMarketplaceOfferStatus::Accepted => Self::Accepted,
            DbMarketplaceOfferStatus::Canceled => Self::Canceled,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct DbMarketplaceOfferRow {
    offer_id: Uuid,
    owner_player_id: Uuid,
    owner_village_id: i32,
    offer_resources: Json<parabellum_types::common::ResourceQuantity>,
    seek_resources: Json<parabellum_types::common::ResourceQuantity>,
    merchants_reserved: i16,
    status: DbMarketplaceOfferStatus,
    accepted_by_player_id: Option<Uuid>,
    accepted_by_village_id: Option<i32>,
    created_at: DateTime<Utc>,
    accepted_at: Option<DateTime<Utc>>,
    canceled_at: Option<DateTime<Utc>>,
}

impl From<DbMarketplaceOfferRow> for MarketplaceOfferModel {
    fn from(row: DbMarketplaceOfferRow) -> Self {
        Self {
            offer_id: row.offer_id,
            owner_player_id: row.owner_player_id,
            owner_village_id: row.owner_village_id as u32,
            offer_resources: row.offer_resources.0,
            seek_resources: row.seek_resources.0,
            merchants_reserved: row.merchants_reserved as u8,
            status: row.status.into(),
            accepted_by_player_id: row.accepted_by_player_id,
            accepted_by_village_id: row.accepted_by_village_id.map(|v| v as u32),
            created_at: row.created_at,
            accepted_at: row.accepted_at,
            canceled_at: row.canceled_at,
        }
    }
}

#[async_trait::async_trait]
impl MarketplaceRepository for PostgresMarketplaceRepository {
    async fn upsert(&self, offer: &MarketplaceOfferModel) -> Result<(), ApplicationError> {
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
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn get_by_offer_id(
        &self,
        offer_id: Uuid,
    ) -> Result<MarketplaceOfferModel, ApplicationError> {
        let row: DbMarketplaceOfferRow = sqlx::query_as(
            r#"
            SELECT offer_id, owner_player_id, owner_village_id, offer_resources, seek_resources,
                   merchants_reserved, status, accepted_by_player_id, accepted_by_village_id,
                   created_at, accepted_at, canceled_at
            FROM rm_marketplace_offers
            WHERE offer_id = $1
            "#,
        )
        .bind(offer_id)
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
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn list_by_owner_village_id(
        &self,
        village_id: u32,
    ) -> Result<Vec<MarketplaceOfferModel>, ApplicationError> {
        let rows: Vec<DbMarketplaceOfferRow> = sqlx::query_as(
            r#"
            SELECT offer_id, owner_player_id, owner_village_id, offer_resources, seek_resources,
                   merchants_reserved, status, accepted_by_player_id, accepted_by_village_id,
                   created_at, accepted_at, canceled_at
            FROM rm_marketplace_offers
            WHERE owner_village_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_open(&self) -> Result<Vec<MarketplaceOfferModel>, ApplicationError> {
        let rows: Vec<DbMarketplaceOfferRow> = sqlx::query_as(
            r#"
            SELECT offer_id, owner_player_id, owner_village_id, offer_resources, seek_resources,
                   merchants_reserved, status, accepted_by_player_id, accepted_by_village_id,
                   created_at, accepted_at, canceled_at
            FROM rm_marketplace_offers
            WHERE status = 'open'
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn claim_open_for_accept(
        &self,
        offer_id: Uuid,
        accepted_by_player_id: Uuid,
        accepted_by_village_id: u32,
        at: DateTime<Utc>,
    ) -> Result<Option<MarketplaceOfferModel>, ApplicationError> {
        let row: Option<DbMarketplaceOfferRow> = sqlx::query_as(
            r#"
            UPDATE rm_marketplace_offers
            SET status = 'accepted',
                accepted_by_player_id = $2,
                accepted_by_village_id = $3,
                accepted_at = $4
            WHERE offer_id = $1
              AND status = 'open'
            RETURNING offer_id, owner_player_id, owner_village_id, offer_resources, seek_resources,
                      merchants_reserved, status, accepted_by_player_id, accepted_by_village_id,
                      created_at, accepted_at, canceled_at
            "#,
        )
        .bind(offer_id)
        .bind(accepted_by_player_id)
        .bind(accepted_by_village_id as i32)
        .bind(at)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(row.map(Into::into))
    }

    async fn list_active_outgoing(
        &self,
        village_id: u32,
    ) -> Result<Vec<MerchantMovement>, ApplicationError> {
        #[derive(Debug, FromRow)]
        struct DbMerchantArrivalRow {
            id: Uuid,
            source_village_id: i32,
            target_village_id: i32,
            resources: serde_json::Value,
            merchants_used: i16,
            arrives_at: DateTime<Utc>,
        }
        #[derive(Debug, FromRow)]
        struct DbMerchantReturnRow {
            id: Uuid,
            source_village_id: i32,
            target_village_id: i32,
            merchants_used: i16,
            arrives_at: DateTime<Utc>,
        }

        let arrivals: Vec<DbMerchantArrivalRow> = sqlx::query_as(
            r#"
            SELECT
                id,
                (payload->>'source_village_id')::int AS source_village_id,
                (payload->>'target_village_id')::int AS target_village_id,
                payload->'resources' AS resources,
                (payload->>'merchants_used')::smallint AS merchants_used,
                (payload->>'arrives_at')::timestamptz AS arrives_at
            FROM rm_scheduled_actions
            WHERE action_type = 'MerchantArrival'
              AND (payload->>'village_id')::int = $1
              AND status IN ('pending', 'processing')
            ORDER BY execute_at ASC, created_at ASC
            "#,
        )
        .bind(village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let returns: Vec<DbMerchantReturnRow> = sqlx::query_as(
            r#"
            SELECT
                id,
                (payload->>'source_village_id')::int AS source_village_id,
                (payload->>'village_id')::int AS target_village_id,
                (payload->>'merchants_used')::smallint AS merchants_used,
                (payload->>'returns_at')::timestamptz AS arrives_at
            FROM rm_scheduled_actions
            WHERE action_type = 'MerchantReturn'
              AND (payload->>'village_id')::int = $1
              AND status IN ('pending', 'processing')
            ORDER BY execute_at ASC, created_at ASC
            "#,
        )
        .bind(village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let mut out = Vec::with_capacity(arrivals.len() + returns.len());
        for row in arrivals {
            let resources: ResourceGroup = serde_json::from_value(row.resources)
                .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
            out.push(MerchantMovement {
                job_id: row.id,
                kind: MerchantMovementKind::Going,
                origin_village_id: row.source_village_id as u32,
                destination_village_id: row.target_village_id as u32,
                resources,
                merchants_used: row.merchants_used as u8,
                arrives_at: row.arrives_at,
            });
        }
        for row in returns {
            out.push(MerchantMovement {
                job_id: row.id,
                kind: MerchantMovementKind::Return,
                origin_village_id: row.source_village_id as u32,
                destination_village_id: row.target_village_id as u32,
                resources: ResourceGroup::new(0, 0, 0, 0),
                merchants_used: row.merchants_used as u8,
                arrives_at: row.arrives_at,
            });
        }
        out.sort_by_key(|m| m.arrives_at);
        Ok(out)
    }

    async fn list_active_incoming(
        &self,
        village_id: u32,
    ) -> Result<Vec<MerchantMovement>, ApplicationError> {
        #[derive(Debug, FromRow)]
        struct DbMerchantArrivalRow {
            id: Uuid,
            source_village_id: i32,
            target_village_id: i32,
            resources: serde_json::Value,
            merchants_used: i16,
            arrives_at: DateTime<Utc>,
        }
        let arrivals: Vec<DbMerchantArrivalRow> = sqlx::query_as(
            r#"
            SELECT
                id,
                (payload->>'source_village_id')::int AS source_village_id,
                (payload->>'target_village_id')::int AS target_village_id,
                payload->'resources' AS resources,
                (payload->>'merchants_used')::smallint AS merchants_used,
                (payload->>'arrives_at')::timestamptz AS arrives_at
            FROM rm_scheduled_actions
            WHERE action_type = 'MerchantArrival'
              AND (payload->>'target_village_id')::int = $1
              AND status IN ('pending', 'processing')
            ORDER BY execute_at ASC, created_at ASC
            "#,
        )
        .bind(village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let mut out = Vec::with_capacity(arrivals.len());
        for row in arrivals {
            let resources: ResourceGroup = serde_json::from_value(row.resources)
                .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
            out.push(MerchantMovement {
                job_id: row.id,
                kind: MerchantMovementKind::Going,
                origin_village_id: row.source_village_id as u32,
                destination_village_id: row.target_village_id as u32,
                resources,
                merchants_used: row.merchants_used as u8,
                arrives_at: row.arrives_at,
            });
        }
        Ok(out)
    }
}
