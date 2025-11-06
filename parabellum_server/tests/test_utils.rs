#[cfg(test)]
pub mod tests {
    use async_trait::async_trait;

    use parabellum_db::{
        PostgresArmyRepository, PostgresJobRepository, PostgresMapRepository,
        PostgresMarketplaceRepository, PostgresPlayerRepository, PostgresVillageRepository,
    };
    use sqlx::{Postgres, Transaction};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use parabellum_app::{
        repository::{
            ArmyRepository, JobRepository, MapRepository, MarketplaceRepository, PlayerRepository,
            VillageRepository,
        },
        uow::{UnitOfWork, UnitOfWorkProvider},
    };
    use parabellum_core::{ApplicationError, Result};

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
            Arc::new(PostgresPlayerRepository::new(self.tx.clone()))
        }

        fn villages(&self) -> Arc<dyn VillageRepository + 'p> {
            Arc::new(PostgresVillageRepository::new(self.tx.clone()))
        }

        fn armies(&self) -> Arc<dyn ArmyRepository + 'p> {
            Arc::new(PostgresArmyRepository::new(self.tx.clone()))
        }

        fn jobs(&self) -> Arc<dyn JobRepository + 'p> {
            Arc::new(PostgresJobRepository::new(self.tx.clone()))
        }

        fn map(&self) -> Arc<dyn MapRepository + 'p> {
            Arc::new(PostgresMapRepository::new(self.tx.clone()))
        }

        fn marketplace(&self) -> Arc<dyn MarketplaceRepository + 'p> {
            Arc::new(PostgresMarketplaceRepository::new(self.tx.clone()))
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
}
