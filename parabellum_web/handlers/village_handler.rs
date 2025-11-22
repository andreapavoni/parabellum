use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::SignedCookieJar;

use crate::{
    handlers::render_template,
    http::AppState,
    templates::{ResourcesTemplate, VillageTemplate},
};

pub async fn village(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Some(_cookie) = jar.get("user_email") {
        // TODO: query logged in user using email from cookie.value().to_string()
        // state.app_bus.query(query, handler)
        let template = VillageTemplate { current_user: true };
        return render_template(template, None).into_response();
    }
    return Redirect::to("/login").into_response();
}

pub async fn resources(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Some(_cookie) = jar.get("user_email") {
        // TODO: query logged in user using email from cookie.value().to_string()
        let template = ResourcesTemplate { current_user: true };
        return render_template(template, None).into_response();
    }
    return Redirect::to("/login").into_response();
}
