use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::SignedCookieJar;

use crate::{
    handlers::{ensure_not_authenticated, render_template},
    http::AppState,
    templates::{HomeTemplate, TemplateLayout},
};

pub async fn home(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Err(redirect) = ensure_not_authenticated(&jar) {
        return redirect.into_response();
    }

    let template = HomeTemplate {
        layout: TemplateLayout::new(None, "home"),
    };
    return render_template(template, None).into_response();
}
