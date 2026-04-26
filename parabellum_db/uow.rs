use sqlx::PgPool;
use std::sync::Arc;

use parabellum_app::{
    repository::*,
    uow::{UnitOfWork, UnitOfWorkProvider},
};
use parabellum_types::errors::ApplicationError;

use crate::{
    persistence::{SharedTx, begin_transaction, commit_transaction, rollback_transaction},
    repository::*,
};

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
    async fn tx<'p>(&'p self) -> Result<Box<dyn UnitOfWork<'p> + 'p>, ApplicationError> {
        let tx_arc = begin_transaction(&self.pool).await?;
        Ok(Box::new(PostgresUnitOfWork { tx: tx_arc }))
    }
}

#[derive(Debug, Clone)]
pub struct PostgresUnitOfWork<'a> {
    tx: SharedTx<'a>,
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

    fn reports(&self) -> Arc<dyn ReportRepository + 'a> {
        Arc::new(PostgresReportRepository::new(self.tx.clone()))
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
        commit_transaction(self.tx).await
    }

    async fn rollback(self: Box<Self>) -> Result<(), ApplicationError> {
        rollback_transaction(self.tx).await
    }
}
