use askama::Template;
use axum::response::{Html, IntoResponse};
use axum::{extract::State, http::StatusCode};

use crate::{http::AppState, templates::HelloTemplate};

pub async fn home_handler(State(_state): State<AppState>) -> impl IntoResponse {
    let template = HelloTemplate {};

    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => {
            tracing::error!("Template error: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response()
        }
    }
}
