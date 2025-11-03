use async_trait::async_trait;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;

use parabellum::{
    Result,
    db::{
        PostgresArmyRepository, PostgresJobRepository, PostgresMapRepository,
        PostgresPlayerRepository, PostgresVillageRepository,
    },
    error::ApplicationError,
    repository::{
        ArmyRepository, JobRepository, MapRepository, PlayerRepository, VillageRepository,
        uow::{UnitOfWork, UnitOfWorkProvider},
    },
};

#[derive(Clone)]
pub struct TestUnitOfWork<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

#[async_trait]
impl<'a, 'p> UnitOfWork<'p> for TestUnitOfWork<'a>
where
    'a: 'p,
{
    fn players(&self) -> Arc<dyn PlayerRepository + 'p> {
        let repo_with_a: Arc<dyn PlayerRepository + 'a> =
            Arc::new(PostgresPlayerRepository::new(self.tx.clone()));

        repo_with_a
    }

    fn villages(&self) -> Arc<dyn VillageRepository + 'p> {
        let repo_with_a: Arc<dyn VillageRepository + 'a> =
            Arc::new(PostgresVillageRepository::new(self.tx.clone()));
        repo_with_a
    }

    fn armies(&self) -> Arc<dyn ArmyRepository + 'p> {
        let repo_with_a: Arc<dyn ArmyRepository + 'a> =
            Arc::new(PostgresArmyRepository::new(self.tx.clone()));
        repo_with_a
    }

    fn jobs(&self) -> Arc<dyn JobRepository + 'p> {
        let repo_with_a: Arc<dyn JobRepository + 'a> =
            Arc::new(PostgresJobRepository::new(self.tx.clone()));
        repo_with_a
    }

    fn map(&self) -> Arc<dyn MapRepository + 'p> {
        let repo_with_a: Arc<dyn MapRepository + 'a> =
            Arc::new(PostgresMapRepository::new(self.tx.clone()));
        repo_with_a
    }

    async fn commit(self: Box<Self>) -> Result<(), ApplicationError> {
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> Result<(), ApplicationError> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct TestUnitOfWorkProvider<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> TestUnitOfWorkProvider<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl<'a> UnitOfWorkProvider for TestUnitOfWorkProvider<'a> {
    async fn begin<'p>(&'p self) -> Result<Box<dyn UnitOfWork<'p> + 'p>, ApplicationError>
    where
        'a: 'p,
    {
        let test_uow: TestUnitOfWork<'a> = TestUnitOfWork::<'a> {
            tx: self.tx.clone(),
        };

        Ok(Box::new(test_uow))
    }
}
