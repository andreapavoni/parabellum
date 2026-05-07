//! HTTP router composition for `parabellum_web`.
//!
//! The router has three responsibilities:
//! - serve SPA static assets (`/assets`, `/static`)
//! - expose JSON API under `/api/v1/*`
//! - provide lightweight liveness endpoint (`/health`)
//!
//! API route handlers live in `crate::api::*` modules.

use axum::{
    Router,
    response::IntoResponse,
    routing::{get, post},
};
use sqlx::PgPool;
use std::{io::Error, net::SocketAddr, sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};

use parabellum_app::{application::GameApplication, config::Config};
use parabellum_types::{Result, errors::ApplicationError};

use crate::{
    api::{
        actions::{
            accept_marketplace_offer, add_building, cancel_marketplace_offer,
            create_marketplace_offer, found_village, recall_troops, release_reinforcements,
            research_academy, research_smithy, send_resources, send_troops, train_units,
            upgrade_building,
        },
        auth::{token_login, token_logout, token_refresh, token_register},
        buildings::building_detail,
        game::{
            map_field, map_region, me_context, me_session, player_profile, report_detail, reports,
            stats, switch_village, village_overview, village_resources,
        },
    },
    auth_tokens::AuthTokenService,
    web::{health::health, spa::spa_shell},
};

#[derive(Clone)]
/// Shared Axum application state.
pub struct AppState {
    pub game_app: Arc<GameApplication>,
    pub db_pool: PgPool,
    pub token_service: Arc<AuthTokenService>,
    pub world_size: i32,
    pub server_speed: i8,
}

impl AppState {
    /// Builds a new `AppState` from app bus, db pool and runtime config.
    pub fn new(game_app: Arc<GameApplication>, db_pool: PgPool, config: &Config) -> AppState {
        let token_service = Arc::new(AuthTokenService::new(config));

        AppState {
            game_app,
            db_pool,
            token_service,
            world_size: config.world_size as i32,
            server_speed: config.speed,
        }
    }
}

pub struct WebRouter {}

impl WebRouter {
    /// Starts the HTTP server and blocks until shutdown/error.
    pub async fn serve(state: AppState, port: u16) -> Result<(), ApplicationError> {
        // Set default locale. We initialize with user locale later
        rust_i18n::set_locale("en-EN");
        // rust_i18n::set_locale("it-IT");

        state
            .token_service
            .ensure_refresh_schema(&state.db_pool)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

        let api_routes = Router::new()
            .route("/auth/token/login", post(token_login))
            .route("/auth/token/register", post(token_register))
            .route("/auth/refresh", post(token_refresh))
            .route("/auth/token/logout", post(token_logout))
            .route("/me/session", get(me_session))
            .route("/me/context", get(me_context))
            .route("/villages/{id}/overview", get(village_overview))
            .route("/villages/{id}/resources", get(village_resources))
            .route("/buildings/{slot_id}", get(building_detail))
            .route("/me/village/current", post(switch_village))
            .route("/buildings/add", post(add_building))
            .route("/buildings/upgrade", post(upgrade_building))
            .route("/army/train", post(train_units))
            .route("/army/send", post(send_troops))
            .route("/army/recall", post(recall_troops))
            .route("/army/release", post(release_reinforcements))
            .route("/marketplace/send", post(send_resources))
            .route("/marketplace/offers", post(create_marketplace_offer))
            .route(
                "/marketplace/offers/{offer_id}/accept",
                post(accept_marketplace_offer),
            )
            .route(
                "/marketplace/offers/{offer_id}/cancel",
                post(cancel_marketplace_offer),
            )
            .route("/academy/research", post(research_academy))
            .route("/smithy/research", post(research_smithy))
            .route("/map/found-village", post(found_village))
            .route("/map/region", get(map_region))
            .route("/map/fields/{id}", get(map_field))
            .route("/reports", get(reports))
            .route("/reports/{id}", get(report_detail))
            .route("/players/{id}", get(player_profile))
            .route("/stats", get(stats))
            .fallback(api_not_found);

        let router = Router::new()
            .nest_service("/assets", ServeDir::new("frontend/assets"))
            .nest_service("/static", ServeDir::new("frontend/static"))
            .nest("/api/v1", api_routes)
            .route("/health", get(health))
            .route("/", get(spa_shell))
            .fallback(get(spa_shell))
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

async fn api_not_found() -> impl IntoResponse {
    crate::api::errors::ApiError::not_found("API route not found")
}
