use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use axum_extra::extract::SignedCookieJar;
use chrono::Utc;
use dioxus::prelude::*;

use crate::{
    components::{LayoutData, PageLayout, wrap_in_html},
    handlers::helpers::ensure_not_authenticated,
    http::AppState,
    pages::HomePage,
};

/// GET / - Home page
pub async fn home_page(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Err(redirect) = ensure_not_authenticated(&jar) {
        return redirect.into_response();
    }

    let layout_data = LayoutData {
        player: None,
        village: None,
        server_time: Utc::now().timestamp(),
        nav_active: "".to_string(),
    };

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            HomePage {}
        }
    });

    Html(wrap_in_html(&body_content)).into_response()
}
