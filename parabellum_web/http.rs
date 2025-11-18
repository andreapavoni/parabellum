use axum::{Router, extract::FromRef, routing::get};
use axum_extra::extract::cookie::Key;
use std::{io::Error, net::SocketAddr, sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};

use parabellum_app::{app::AppBus, config::Config};
use parabellum_core::{ApplicationError, Result};

use crate::handlers::{home_handler, login, login_page, logout, register, register_page};

#[derive(Clone)]
pub struct AppState {
    pub app_bus: Arc<AppBus>,
    pub cookie_key: Key,
}

impl AppState {
    pub fn new(app_bus: Arc<AppBus>, config: &Config) -> AppState {
        let cookie_key = Key::from(config.auth_cookie_secret.as_bytes());

        AppState {
            app_bus,
            cookie_key,
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
        let router = Router::new()
            .nest_service("/assets", ServeDir::new("parabellum_web/assets"))
            .route("/", get(home_handler))
            .route("/login", get(login_page).post(login))
            .route("/register", get(register_page).post(register))
            .route("/logout", get(logout))
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
