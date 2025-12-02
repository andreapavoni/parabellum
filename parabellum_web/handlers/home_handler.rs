use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::SignedCookieJar;

use crate::{
    handlers::{ensure_not_authenticated, render_template},
    http::AppState,
    templates::HomeTemplate,
    view_helpers::server_time,
};

pub async fn home(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Err(redirect) = ensure_not_authenticated(&jar) {
        return redirect.into_response();
    }

    let template = HomeTemplate {
        current_user: None,
        nav_active: "home",
        server_time: server_time(),
    };
    return render_template(template, None).into_response();
}
