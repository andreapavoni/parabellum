use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::repository::*;

/// A Unit of Work (UoW) works as a provider for repositories
/// that all operate within a single transaction.
#[async_trait::async_trait]
pub trait UnitOfWork<'a>: Send + Sync {
    // Methods to access transactional repositories
    fn players(&self) -> Arc<dyn PlayerRepository + 'a>;
    fn villages(&self) -> Arc<dyn VillageRepository + 'a>;
    fn armies(&self) -> Arc<dyn ArmyRepository + 'a>;
    fn jobs(&self) -> Arc<dyn JobRepository + 'a>;
    fn map(&self) -> Arc<dyn MapRepository + 'a>;
    fn marketplace(&self) -> Arc<dyn MarketplaceRepository + 'a>;
    fn heroes(&self) -> Arc<dyn HeroRepository + 'a>;
    fn users(&self) -> Arc<dyn UserRepository + 'a>;

    // Transaction control methods
    // Consume self to ensure the UoW is not used after commit/rollback
    async fn commit(self: Box<Self>) -> Result<(), ApplicationError>;
    async fn rollback(self: Box<Self>) -> Result<(), ApplicationError>;
}

/// A factory for creating Unit of Work instances.
#[async_trait::async_trait]
pub trait UnitOfWorkProvider: Send + Sync {
    /// Begin a new Unit of Work (transaction).
    async fn tx<'p>(&'p self) -> Result<Box<dyn UnitOfWork<'p> + 'p>, ApplicationError>;
    // You might also want to provide a way to get a repository pool
    // for non-transactional operations like read-only operations,
    // async fn pool(&self) -> Arc<dyn SomeRepoPool>;
}
