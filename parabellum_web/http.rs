use axum::{
    Router,
    extract::FromRef,
    routing::{get, post},
};
use axum_extra::extract::cookie::Key;
use std::{io::Error, net::SocketAddr, sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};

use parabellum_app::{app::AppBus, config::Config};
use parabellum_types::{Result, errors::ApplicationError};

use crate::handlers::{
    army::{send_troops, train_units},
    auth::{login, login_page, logout, register, register_page},
    building::{build, building_page},
    buildings::{research_smithy, research_unit},
    home::home_page,
    map::{map_page, map_region},
    reports::{report_page, reports_page},
    resources::resources_page,
    village::village_page,
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
            .route("/", get(home_page))
            .route("/login", get(login_page).post(login))
            .route("/register", get(register_page).post(register));

        // Protected routes (require authenticated user)
        let protected_routes = Router::new()
            // Dioxus routes (primary)
            .route("/village", get(village_page))
            .route("/resources", get(resources_page))
            .route("/map", get(map_page))
            .route("/map/data", get(map_region))
            .route("/reports", get(reports_page))
            .route("/reports/{id}", get(report_page))
            .route("/build/{slot_id}", get(building_page).post(build))
            .route("/army/train", post(train_units))
            .route("/army/send", post(send_troops))
            .route("/academy/research", post(research_unit))
            .route("/smithy/research", post(research_smithy))
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
