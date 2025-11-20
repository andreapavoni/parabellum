use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::SignedCookieJar;

use crate::{handlers::render_template, http::AppState, templates::HelloTemplate};

pub async fn home_handler(
    State(_state): State<AppState>,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    let (is_logged_in, user_email) = match jar.get("user_email") {
        Some(cookie) => (true, Some(cookie.value().to_string())),
        None => (false, None),
    };
    let template = HelloTemplate {
        current_user: is_logged_in,
        current_user_email: user_email,
    };
    return render_template(template, None).into_response();
}
