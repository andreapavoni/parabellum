use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use axum_extra::extract::SignedCookieJar;
use dioxus::prelude::*;

use crate::{
    components::wrap_in_html, handlers::helpers::ensure_not_authenticated, http::AppState,
    pages::HomePage,
};

/// GET / - Home page
pub async fn home_page(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Err(redirect) = ensure_not_authenticated(&jar) {
        return redirect.into_response();
    }

    // Landing page doesn't use PageLayout - it has its own full-page design
    let body_content = dioxus_ssr::render_element(rsx! {
        HomePage {}
    });

    Html(wrap_in_html(&body_content)).into_response()
}
