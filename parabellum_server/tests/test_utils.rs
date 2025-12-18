#[cfg(test)]
pub mod tests {
    use async_trait::async_trait;
    use axum::http::{HeaderValue, StatusCode};
    use parabellum_web::{AppState, WebRouter};
    use rand::Rng;
    use reqwest::{Client, header, redirect::Policy};
    use sqlx::{Postgres, Transaction};
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::Mutex;

    use parabellum_app::{
        app::AppBus,
        auth::hash_password,
        config::Config,
        job_registry::AppJobRegistry,
        jobs::worker::JobWorker,
        repository::{
            ArmyRepository, HeroRepository, JobRepository, MapRepository, MarketplaceRepository,
            PlayerRepository, ReportRepository, UserRepository, VillageRepository,
        },
        uow::{UnitOfWork, UnitOfWorkProvider},
    };
    use parabellum_db::{
        PostgresArmyRepository, PostgresHeroRepository, PostgresJobRepository,
        PostgresMapRepository, PostgresMarketplaceRepository, PostgresPlayerRepository,
        PostgresReportRepository, PostgresUserRepository, PostgresVillageRepository,
        bootstrap_world_map, establish_test_connection_pool,
    };
    use parabellum_game::{
        models::{army::Army, hero::Hero, village::Village},
        test_utils::{
            ArmyFactoryOptions, PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions,
            army_factory, player_factory, valley_factory, village_factory,
        },
    };
    use parabellum_types::{Result, army::TroopSet, errors::ApplicationError};
    use parabellum_types::{
        common::{Player, User},
        map::Position,
        tribe::Tribe,
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

        fn reports(&self) -> Arc<dyn ReportRepository + 'p> {
            Arc::new(PostgresReportRepository::new(self.tx.clone()))
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
        async fn tx<'p>(&'p self) -> Result<Box<dyn UnitOfWork<'p> + 'p>, ApplicationError>
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
    pub async fn setup_app(
        world_map: bool,
    ) -> Result<(
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

        if world_map {
            bootstrap_world_map(&pool, config.world_size).await?;
        }

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
    pub async fn setup_web_app() -> Result<Arc<dyn UnitOfWorkProvider>> {
        let (app_bus, _, uow_provider, config) = setup_app(true).await?;
        let app = Arc::new(app_bus);
        let state = AppState::new(app, &config);
        tokio::spawn(WebRouter::serve(state.clone(), 8088));

        Ok(uow_provider)
    }

    #[allow(dead_code)]
    pub async fn setup_user_cookie(user: User) -> HeaderValue {
        let client = setup_http_client(None, None).await;
        let csrf_token = fetch_csrf_token(&client, "http://localhost:8088/login")
            .await
            .expect("Failed to fetch CSRF token");
        let mut form = HashMap::new();
        form.insert("email", user.email.as_str());
        form.insert("password", "parabellum!");
        form.insert("csrf_token", csrf_token.as_str());
        let res = client
            .post("http://localhost:8088/login")
            .form(&form)
            .send()
            .await
            .unwrap();

        let cookies = res.headers().get_all("set-cookie");
        let mut cookie_pairs = Vec::new();
        for value in cookies.iter() {
            if let Ok(val_str) = value.to_str() {
                if let Some((pair, _)) = val_str.split_once(';') {
                    cookie_pairs.push(pair.to_string());
                } else {
                    cookie_pairs.push(val_str.to_string());
                }
            }
        }

        if cookie_pairs.is_empty() {
            panic!(
                "setup cookie failed: {:#?}",
                res.text().await.unwrap().to_string()
            );
        }

        let header_value = cookie_pairs.join("; ");
        HeaderValue::from_str(&header_value).unwrap()
    }

    #[allow(dead_code)]
    pub async fn setup_http_client(cookie: Option<HeaderValue>, redirects: Option<u8>) -> Client {
        let redirect_policy = redirects.map_or(Policy::none(), |n| Policy::limited(n as usize));
        let client = Client::builder()
            .redirect(redirect_policy)
            .cookie_store(true);

        if cookie.is_none() {
            return client.build().unwrap();
        }

        let cookie = cookie.unwrap();
        let mut request_headers = header::HeaderMap::new();
        request_headers.insert(header::COOKIE, cookie);
        client.default_headers(request_headers).build().unwrap()
    }

    /// Fetches a CSRF token from a form page (login or register).
    /// Makes a GET request to the page, parses the HTML to extract the CSRF token
    /// from the hidden input field, and returns it along with the client (which has
    /// the CSRF cookie stored).
    #[allow(dead_code)]
    pub async fn fetch_csrf_token(
        client: &Client,
        page_url: &str,
    ) -> Result<String, ApplicationError> {
        let res = client.get(page_url).send().await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body = res.text().await.unwrap();
        // Parse HTML to find: <input type="hidden" name="csrf_token" value="...">
        if let Some(start) = body.find(r#"name="csrf_token" value=""#) {
            let value_start = start + r#"name="csrf_token" value=""#.len();
            if let Some(end) = body[value_start..].find('"') {
                let token = body[value_start..value_start + end].to_string();
                return Ok(token);
            }
        }

        Err(ApplicationError::Infrastructure(
            "Failed to extract CSRF token from form page".to_string(),
        ))
    }

    #[allow(dead_code)]
    pub async fn setup_player_party(
        uow_provider: Arc<dyn UnitOfWorkProvider>,
        position: Option<Position>,
        tribe: Tribe,
        units: TroopSet,
        with_hero: bool,
    ) -> Result<(Player, Village, Army, Option<Hero>, User)> {
        let uow = uow_provider.tx().await?;
        let player: Player;
        let village: Village;
        let army: Army;
        let hero: Option<Hero>;
        let user: User;
        {
            let player_repo = uow.players();
            let village_repo = uow.villages();
            let army_repo = uow.armies();
            let hero_repo = uow.heroes();
            let user_repo = uow.users();

            let mut rng = rand::thread_rng();
            let position = position.unwrap_or_else(|| {
                let x = rng.gen_range(1..99);
                let y = rng.gen_range(1..99);
                Position { x, y }
            });

            let email = format!("parabellum-{}@example.com", rng.gen_range(1..99999));
            user_repo
                .save(email.clone(), hash_password("parabellum!")?)
                .await?;
            user = user_repo.get_by_email(&email).await?;

            player = player_factory(PlayerFactoryOptions {
                tribe: Some(tribe.clone()),
                user_id: Some(user.id),
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

        Ok((player, village, army, hero, user))
    }
}
