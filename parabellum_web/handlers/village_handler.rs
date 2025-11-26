use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::SignedCookieJar;

use crate::{
    handlers::{current_user, render_template},
    http::AppState,
    templates::{ResourcesTemplate, VillageTemplate},
};

pub async fn village(State(state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    match current_user(&state, &jar).await {
        Ok(_user) => {
            let template = VillageTemplate { current_user: true };
            render_template(template, None).into_response()
        }
        Err(redirect) => redirect.into_response(),
    }
}

pub async fn resources(State(state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    match current_user(&state, &jar).await {
        Ok(_user) => {
            let template = ResourcesTemplate { current_user: true };
            render_template(template, None).into_response()
        }
        Err(redirect) => redirect.into_response(),
    }
}
