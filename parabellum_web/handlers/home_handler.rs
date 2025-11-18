use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_extra::extract::SignedCookieJar;

use crate::{http::AppState, templates::HelloTemplate};

pub async fn home_handler(
    State(_state): State<AppState>,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    // Check for a signed auth cookie
    // FIXME use SignedCookieJar with state.hash
    let (is_logged_in, user_email) = match jar.get("user_email") {
        Some(cookie) => (true, Some(cookie.value().to_string())),
        None => (false, None),
    };
    let template = HelloTemplate {
        current_user: is_logged_in,
        current_user_email: user_email,
    };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => {
            tracing::error!("Template error: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response()
        }
    }
}
