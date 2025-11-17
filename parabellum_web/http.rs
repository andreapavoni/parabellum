use axum::{Router, routing::get};
use std::{io::Error, net::SocketAddr};
use tower_http::{services::ServeDir, trace::TraceLayer};

use parabellum_app::app::AppState;
use parabellum_core::{ApplicationError, Result};

use crate::handlers::home_handler;

pub struct WebRouter {}

impl WebRouter {
    pub async fn serve(state: AppState, port: u16) -> Result<(), ApplicationError> {
        let router = Router::new()
            .nest_service("/assets", ServeDir::new("parabellum_web/assets"))
            .route("/", get(home_handler))
            .with_state(state)
            .layer(TraceLayer::new_for_http());

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            let err = format!("{:#?}", e);
            ApplicationError::Infrastructure(err)
        })?;

        axum::serve(listener, router).await.map_err(infra_error)?;

        Ok(())
    }
}

fn infra_error(e: Error) -> ApplicationError {
    let err = format!("{:#?}", e);
    ApplicationError::Infrastructure(err)
}
