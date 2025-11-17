#[cfg(test)]
pub mod tests {
    use async_trait::async_trait;
    use rand::Rng;

    use parabellum_db::{
        PostgresArmyRepository, PostgresHeroRepository, PostgresJobRepository,
        PostgresMapRepository, PostgresMarketplaceRepository, PostgresPlayerRepository,
        PostgresUserRepository, PostgresVillageRepository, establish_test_connection_pool,
    };
    use parabellum_game::{
        models::{
            army::{Army, TroopSet},
            hero::Hero,
            village::Village,
        },
        test_utils::{
            ArmyFactoryOptions, PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions,
            army_factory, player_factory, valley_factory, village_factory,
        },
    };
    use parabellum_types::{common::Player, map::Position, tribe::Tribe};
    use sqlx::{Postgres, Transaction};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use parabellum_app::{
        app::AppBus,
        config::Config,
        job_registry::AppJobRegistry,
        jobs::worker::JobWorker,
        repository::{
            ArmyRepository, HeroRepository, JobRepository, MapRepository, MarketplaceRepository,
            PlayerRepository, UserRepository, VillageRepository,
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

        fn heroes(&self) -> Arc<dyn HeroRepository + 'p> {
            Arc::new(PostgresHeroRepository::new(self.tx.clone()))
        }

        fn users(&self) -> Arc<dyn UserRepository + 'p> {
            Arc::new(PostgresUserRepository::new(self.tx.clone()))
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

    #[allow(dead_code)]
    pub async fn setup_app() -> Result<(
        AppBus,
        Arc<JobWorker>,
        Arc<dyn UnitOfWorkProvider>,
        Arc<Config>,
    )> {
        let config = Arc::new(Config::from_env());
        let pool = establish_test_connection_pool().await.unwrap();
        let master_tx = pool.begin().await.unwrap();
        let master_tx_arc = Arc::new(Mutex::new(master_tx));
        let uow_provider: Arc<dyn UnitOfWorkProvider> =
            Arc::new(TestUnitOfWorkProvider::new(master_tx_arc.clone()));

        let app_bus = AppBus::new(config.clone(), uow_provider.clone());
        let app_registry = Arc::new(AppJobRegistry::new());
        let worker = Arc::new(JobWorker::new(
            uow_provider.clone(),
            app_registry,
            config.clone(),
        ));

        Ok((app_bus, worker, uow_provider, config))
    }

    #[allow(dead_code)]
    pub async fn setup_player_party(
        uow_provider: Arc<dyn UnitOfWorkProvider>,
        position: Option<Position>,
        tribe: Tribe,
        units: TroopSet,
        with_hero: bool,
    ) -> Result<(Player, Village, Army, Option<Hero>)> {
        let uow = uow_provider.begin().await?;
        let player: Player;
        let village: Village;
        let army: Army;
        let hero: Option<Hero>;
        {
            let player_repo = uow.players();
            let village_repo = uow.villages();
            let army_repo = uow.armies();
            let hero_repo = uow.heroes();

            let position = position.unwrap_or_else(|| {
                let mut rng = rand::thread_rng();
                let x = rng.gen_range(1..99);
                let y = rng.gen_range(1..99);
                Position { x, y }
            });

            player = player_factory(PlayerFactoryOptions {
                tribe: Some(tribe.clone()),
                ..Default::default()
            });
            player_repo.save(&player).await?;

            let valley = valley_factory(ValleyFactoryOptions {
                position: Some(position),
                ..Default::default()
            });
            village = village_factory(VillageFactoryOptions {
                valley: Some(valley),
                player: Some(player.clone()),
                ..Default::default()
            });
            village_repo.save(&village).await?;

            hero = if with_hero {
                let hero = Hero::new(None, village.id, player.id, player.tribe.clone(), None);
                hero_repo.save(&hero).await.unwrap();
                Some(hero)
            } else {
                None
            };

            army = army_factory(ArmyFactoryOptions {
                player_id: Some(player.id),
                village_id: Some(village.id),
                units: Some(units),
                tribe: Some(tribe.clone()),
                hero: hero.clone(),
                ..Default::default()
            });
            army_repo.save(&army).await?;
        }

        uow.commit().await?;

        Ok((player, village, army, hero))
    }
}
