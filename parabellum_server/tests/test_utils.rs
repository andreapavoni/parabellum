#[cfg(test)]
pub mod tests {
    use async_trait::async_trait;
    use axum::http::{HeaderValue, StatusCode};
    use parabellum_web::{AppState, WebRouter};
    use rand::Rng;
    use reqwest::{Client, header, redirect::Policy};
    use serde_json::Value;
    use sqlx::{PgPool, postgres::PgPoolOptions};
    use std::{env, net::TcpListener, sync::Arc, time::Duration};
    use uuid::Uuid;

    use parabellum_app::{
        app::AppBus,
        auth::hash_password,
        config::Config,
        job_registry::AppJobRegistry,
        jobs::worker::JobWorker,
        uow::{UnitOfWork, UnitOfWorkProvider},
    };
    use parabellum_db::{bootstrap_world_map, uow::PostgresUnitOfWorkProvider};
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

    #[allow(dead_code)]
    #[derive(Debug, Clone)]
    pub struct AuthTokens {
        pub access_token: String,
        pub refresh_token: String,
    }

    #[derive(Clone)]
    struct IsolatedSchemaHandle {
        root_url: String,
        schema_name: String,
    }

    impl Drop for IsolatedSchemaHandle {
        fn drop(&mut self) {
            let root_url = self.root_url.clone();
            let schema_name = self.schema_name.clone();
            std::thread::spawn(move || {
                if let Ok(rt) = tokio::runtime::Runtime::new() {
                    rt.block_on(async move {
                        if let Ok(pool) = PgPoolOptions::new()
                            .max_connections(1)
                            .connect(&root_url)
                            .await
                        {
                            let query =
                                format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", schema_name);
                            let _ = sqlx::query(&query).execute(&pool).await;
                        }
                    });
                }
            });
        }
    }

    #[derive(Clone)]
    pub struct TestUnitOfWorkProvider {
        inner: PostgresUnitOfWorkProvider,
        #[allow(dead_code)]
        schema_handle: Arc<IsolatedSchemaHandle>,
    }

    impl TestUnitOfWorkProvider {
        fn new(pool: PgPool, schema_handle: Arc<IsolatedSchemaHandle>) -> Self {
            Self {
                inner: PostgresUnitOfWorkProvider::new(pool),
                schema_handle,
            }
        }
    }

    #[async_trait]
    impl UnitOfWorkProvider for TestUnitOfWorkProvider {
        async fn tx<'p>(&'p self) -> Result<Box<dyn UnitOfWork<'p> + 'p>, ApplicationError> {
            self.inner.tx().await
        }
    }

    fn append_search_path(url: &str, schema_name: &str) -> String {
        let separator = if url.contains('?') { '&' } else { '?' };
        format!("{url}{separator}options=-csearch_path%3D{schema_name}%2Cpublic")
    }

    async fn create_isolated_test_pool() -> Result<(PgPool, Arc<IsolatedSchemaHandle>)> {
        dotenvy::dotenv().ok();
        let root_url = env::var("TEST_DATABASE_URL")
            .map_err(|_| ApplicationError::Unknown("TEST_DATABASE_URL must be set".to_string()))?;
        let schema_name = format!("test_{}", Uuid::new_v4().simple());

        let root_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&root_url)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        sqlx::query(r#"CREATE EXTENSION IF NOT EXISTS "uuid-ossp""#)
            .execute(&root_pool)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let create_query = format!("CREATE SCHEMA \"{}\"", schema_name);
        sqlx::query(&create_query)
            .execute(&root_pool)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let isolated_url = append_search_path(&root_url, &schema_name);
        let isolated_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&isolated_url)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        sqlx::migrate!("../migrations")
            .run(&isolated_pool)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let handle = Arc::new(IsolatedSchemaHandle {
            root_url,
            schema_name,
        });

        Ok((isolated_pool, handle))
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
        let (pool, schema_handle) = create_isolated_test_pool().await?;
        let uow_provider: Arc<dyn UnitOfWorkProvider> =
            Arc::new(TestUnitOfWorkProvider::new(pool.clone(), schema_handle));

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
    pub async fn setup_web_app() -> Result<(Arc<dyn UnitOfWorkProvider>, String)> {
        let (pool, schema_handle) = create_isolated_test_pool().await?;
        let config = Arc::new(Config::from_env());
        bootstrap_world_map(&pool, config.world_size).await?;
        let uow_provider: Arc<dyn UnitOfWorkProvider> =
            Arc::new(TestUnitOfWorkProvider::new(pool.clone(), schema_handle));

        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let port = listener
            .local_addr()
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?
            .port();
        drop(listener);

        let app_bus = AppBus::new(config.clone(), uow_provider.clone());
        let app = Arc::new(app_bus);
        let state = AppState::new(app, pool, &config);
        tokio::spawn(WebRouter::serve(state.clone(), port));

        let base_url = format!("http://127.0.0.1:{port}");
        let client = reqwest::Client::new();
        let health_url = format!("{base_url}/health");
        let mut ready = false;
        for _ in 0..40 {
            if let Ok(response) = client.get(&health_url).send().await
                && response.status() == StatusCode::OK
            {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        if !ready {
            return Err(ApplicationError::Unknown(
                "Web test app did not become ready".to_string(),
            ));
        }

        Ok((uow_provider, base_url))
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

    #[allow(dead_code)]
    pub async fn login_tokens(
        client: &Client,
        base_url: &str,
        email: &str,
        password: &str,
    ) -> AuthTokens {
        let response = client
            .post(format!("{base_url}/api/v1/auth/token/login"))
            .header("content-type", "application/json")
            .body(
                serde_json::json!({
                    "email": email,
                    "password": password,
                })
                .to_string(),
            )
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let payload: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();

        AuthTokens {
            access_token: payload["accessToken"].as_str().unwrap().to_string(),
            refresh_token: payload["refreshToken"].as_str().unwrap().to_string(),
        }
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
