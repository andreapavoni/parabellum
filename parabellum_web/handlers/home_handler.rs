use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::SignedCookieJar;

use crate::{handlers::render_template, http::AppState, templates::HomeTemplate};

pub async fn home(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Some(_cookie) = jar.get("user_email") {
        return Redirect::to("/village").into_response();
    }

    let template = HomeTemplate {
        current_user: false,
    };
    return render_template(template, None).into_response();
}
