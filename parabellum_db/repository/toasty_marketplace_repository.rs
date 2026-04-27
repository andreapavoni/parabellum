use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::MarketplaceRepository;
use parabellum_game::models::marketplace::MarketplaceOffer;
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
};

use crate::toasty_models::marketplace::MarketplaceOfferDbRow;

pub struct ToastyMarketplaceRepository {
    db: Arc<Mutex<toasty::Db>>,
}

impl ToastyMarketplaceRepository {
    pub fn new(db: Arc<Mutex<toasty::Db>>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl MarketplaceRepository for ToastyMarketplaceRepository {
    async fn create(&self, offer: &MarketplaceOffer) -> Result<(), ApplicationError> {
        let record = MarketplaceOfferDbRow::try_from(offer)?;
        let mut tx_guard = self.db.lock().await;

        toasty::create!(MarketplaceOfferDbRow {
            id: record.id,
            player_id: record.player_id,
            village_id: record.village_id,
            offer_resources: record.offer_resources,
            seek_resources: record.seek_resources,
            merchants_required: record.merchants_required,
            created_at: record.created_at,
        })
        .exec(&mut *tx_guard)
        .await
        .map_err(map_toasty_error)?;

        Ok(())
    }

    async fn get_by_id(&self, offer_id: Uuid) -> Result<MarketplaceOffer, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let row = MarketplaceOfferDbRow::get_by_id(&mut *tx_guard, offer_id)
            .await
            .map_err(map_toasty_error)?;
        MarketplaceOffer::try_from(row)
    }

    async fn list_by_village(
        &self,
        village_id: u32,
    ) -> Result<Vec<MarketplaceOffer>, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let mut rows =
            toasty::query!(MarketplaceOfferDbRow filter .village_id == #(village_id as i32))
                .exec(&mut *tx_guard)
                .await
                .map_err(map_toasty_error)?;

        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        rows.into_iter()
            .map(MarketplaceOffer::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn list_all(&self) -> Result<Vec<MarketplaceOffer>, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let mut rows = MarketplaceOfferDbRow::all()
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        rows.into_iter()
            .map(MarketplaceOffer::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn delete(&self, offer_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let row = MarketplaceOfferDbRow::get_by_id(&mut *tx_guard, offer_id)
            .await
            .map_err(map_toasty_error)?;
        row.delete()
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        Ok(())
    }
}

fn map_toasty_error(err: toasty::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Transaction(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::toasty_db::establish_test_toasty_db;

    #[tokio::test]
    async fn toasty_marketplace_repo_crud() -> Result<(), ApplicationError> {
        let pool = crate::establish_test_connection_pool()
            .await
            .map_err(ApplicationError::Db)?;
        let seed: Option<(Uuid, i32)> =
            sqlx::query_as("SELECT v.player_id, v.id FROM villages v LIMIT 1")
                .fetch_optional(&pool)
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        let Some((player_id, village_id)) = seed else {
            return Ok(());
        };

        let toasty_db = Arc::new(Mutex::new(
            establish_test_toasty_db()
                .await
                .map_err(ApplicationError::Db)?,
        ));
        let repo = ToastyMarketplaceRepository::new(toasty_db.clone());

        let offer = MarketplaceOffer::new(
            player_id,
            village_id as u32,
            parabellum_types::common::ResourceGroup::new(10, 0, 0, 0),
            parabellum_types::common::ResourceGroup::new(0, 10, 0, 0),
            1,
        );

        repo.create(&offer).await?;
        let loaded = repo.get_by_id(offer.id).await?;
        assert_eq!(loaded.id, offer.id);

        let village_offers = repo.list_by_village(village_id as u32).await?;
        assert!(village_offers.iter().any(|o| o.id == offer.id));

        let all = repo.list_all().await?;
        assert!(all.iter().any(|o| o.id == offer.id));

        repo.delete(offer.id).await?;
        let after_delete = repo.list_by_village(village_id as u32).await?;
        assert!(!after_delete.iter().any(|o| o.id == offer.id));

        drop(repo);
        drop(toasty_db);
        Ok(())
    }
}
