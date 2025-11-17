use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;

use parabellum_app::{
    repository::*,
    uow::{UnitOfWork, UnitOfWorkProvider},
};
use parabellum_core::{ApplicationError, DbError};

use crate::repository::*;

#[derive(Debug, Clone)]
pub struct PostgresUnitOfWorkProvider {
    pool: PgPool,
}

impl PostgresUnitOfWorkProvider {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UnitOfWorkProvider for PostgresUnitOfWorkProvider {
    async fn begin<'p>(&'p self) -> Result<Box<dyn UnitOfWork<'p> + 'p>, ApplicationError> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        // Transaction must be 'static to be stored in Arc.
        let tx_arc = Arc::new(Mutex::new(tx));

        Ok(Box::new(PostgresUnitOfWork { tx: tx_arc }))
    }
}

#[derive(Debug, Clone)]
pub struct PostgresUnitOfWork<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

#[async_trait::async_trait]
impl<'a> UnitOfWork<'a> for PostgresUnitOfWork<'a> {
    fn players(&self) -> Arc<dyn PlayerRepository + 'a> {
        Arc::new(PostgresPlayerRepository::new(self.tx.clone()))
    }

    fn villages(&self) -> Arc<dyn VillageRepository + 'a> {
        Arc::new(PostgresVillageRepository::new(self.tx.clone()))
    }

    fn armies(&self) -> Arc<dyn ArmyRepository + 'a> {
        Arc::new(PostgresArmyRepository::new(self.tx.clone()))
    }

    fn jobs(&self) -> Arc<dyn JobRepository + 'a> {
        Arc::new(PostgresJobRepository::new(self.tx.clone()))
    }

    fn map(&self) -> Arc<dyn MapRepository + 'a> {
        Arc::new(PostgresMapRepository::new(self.tx.clone()))
    }

    fn marketplace(&self) -> Arc<dyn MarketplaceRepository + 'a> {
        Arc::new(PostgresMarketplaceRepository::new(self.tx.clone()))
    }

    fn heroes(&self) -> Arc<dyn HeroRepository + 'a> {
        Arc::new(PostgresHeroRepository::new(self.tx.clone()))
    }

    fn users(&self) -> Arc<dyn UserRepository + 'a> {
        Arc::new(PostgresUserRepository::new(self.tx.clone()))
    }

    async fn commit(self: Box<Self>) -> Result<(), ApplicationError> {
        // Try to unwrap the Arc to get ownership of the Mutex<Transaction>.
        // If this fails, it means there are other references to the Arc,
        // the transaction cannot be committed (logical error) and will rollback on Drop.
        if let Ok(mutex) = Arc::try_unwrap(self.tx) {
            mutex
                .into_inner()
                .commit()
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        } else {
            return Err(ApplicationError::Db(DbError::Transaction(
                "transaction still has multiple owners".to_string(),
            )));
        }
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> Result<(), ApplicationError> {
        if let Ok(mutex) = Arc::try_unwrap(self.tx) {
            mutex
                .into_inner()
                .rollback()
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        }
        Ok(())
    }
}
