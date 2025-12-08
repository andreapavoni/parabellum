use crate::{
    components::{MapPage, PageLayout, wrap_in_html},
    handlers::CurrentUser,
    http::AppState,
};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use dioxus::prelude::*;

use super::helpers::create_layout_data;

/// Render the map page using Dioxus SSR
pub async fn map(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
    let layout_data = create_layout_data(&user, "map");

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data.clone(),
            MapPage {
                village: layout_data.village.unwrap(),
                world_size: state.world_size
            }
        }
    });

    Html(wrap_in_html(&body_content))
}
