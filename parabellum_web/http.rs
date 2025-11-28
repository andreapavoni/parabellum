use axum::{Router, extract::FromRef, routing::get};
use axum_extra::extract::cookie::Key;
use std::{io::Error, net::SocketAddr, sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};

use parabellum_app::{app::AppBus, config::Config};
use parabellum_types::{Result, errors::ApplicationError};

use crate::handlers::{
    building, home, login, login_page, logout, map, map_region, register, register_page, resources,
    village,
};

#[derive(Clone)]
pub struct AppState {
    pub app_bus: Arc<AppBus>,
    pub cookie_key: Key,
    pub world_size: i32,
    pub server_speed: i8,
}

impl AppState {
    pub fn new(app_bus: Arc<AppBus>, config: &Config) -> AppState {
        let cookie_key = Key::from(config.auth_cookie_secret.as_bytes());

        AppState {
            app_bus,
            cookie_key,
            world_size: config.world_size as i32,
            server_speed: config.speed,
        }
    }
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

pub struct WebRouter {}

impl WebRouter {
    pub async fn serve(state: AppState, port: u16) -> Result<(), ApplicationError> {
        // Set default locale. We initialize with user locale later
        rust_i18n::set_locale("en-EN");
        // rust_i18n::set_locale("it-IT");

        // Public routes (no authentication required)
        let public_routes = Router::new()
            .route("/", get(home))
            .route("/login", get(login_page).post(login))
            .route("/register", get(register_page).post(register));

        // Protected routes (require authenticated user)
        let protected_routes = Router::new()
            .route("/village", get(village))
            .route("/resources", get(resources))
            .route("/build", get(building))
            .route("/map", get(map))
            .route("/map/data", get(map_region))
            .route("/logout", get(logout));

        let router = Router::new()
            .nest_service("/assets", ServeDir::new("frontend/assets"))
            .merge(public_routes)
            .merge(protected_routes)
            .with_state(state)
            .layer(TraceLayer::new_for_http());

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            let err = format!("{:#?}", e);
            ApplicationError::Infrastructure(err)
        })?;

        tracing::info!(
            "HTTP Server started, listening on http://{}",
            addr.to_string()
        );
        axum::serve(listener, router).await.map_err(infra_error)?;

        Ok(())
    }
}

fn infra_error(e: Error) -> ApplicationError {
    let err = format!("{:#?}", e);
    ApplicationError::Infrastructure(err)
}
