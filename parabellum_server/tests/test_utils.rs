#[cfg(test)]
pub mod tests {
    use axum::http::{HeaderValue, StatusCode};
    use parabellum_web::{AppState, WebRouter};
    use reqwest::{Client, header, redirect::Policy};
    use sqlx::{PgPool, postgres::PgPoolOptions};
    use std::{env, net::TcpListener, sync::Arc, time::Duration};
    use uuid::Uuid;

    use parabellum_app::{application::GameApplication, config::Config};
    use parabellum_db::{
        adapters::VillageEsAdapter, bootstrap_world_map, es::VillageEsService,
        identity::IdentityService,
    };
    use parabellum_types::{Result, errors::ApplicationError};

    #[derive(Clone)]
    pub struct IsolatedSchemaHandle {
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
    pub async fn setup_web_app() -> Result<(Arc<IsolatedSchemaHandle>, String)> {
        let (pool, schema_handle) = create_isolated_test_pool().await?;
        let config = Arc::new(Config::from_env());
        bootstrap_world_map(&pool, config.world_size).await?;

        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let port = listener
            .local_addr()
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?
            .port();
        drop(listener);

        let villages = Arc::new(VillageEsAdapter::new(
            VillageEsService::new(pool.clone()),
            config.clone(),
        ));
        let game_app = Arc::new(GameApplication::new(
            Arc::new(IdentityService::new(pool.clone(), config.clone())),
            villages.clone(),
            villages.clone(),
            villages,
        ));
        let state = AppState::new(game_app, pool, &config);
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

        Ok((schema_handle, base_url))
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
}
