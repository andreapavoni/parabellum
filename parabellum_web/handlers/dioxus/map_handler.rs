use crate::{
    components::{MapPage, MapPageData, PageLayout, wrap_in_html},
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
    let data = MapPageData {
        center_x: user.village.position.x,
        center_y: user.village.position.y,
        home_x: user.village.position.x,
        home_y: user.village.position.y,
        home_village_id: user.village.id,
        world_size: state.world_size,
    };

    let layout_data = create_layout_data(&user, "map");

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            MapPage { data: data }
        }
    });

    Html(wrap_in_html(&body_content))
}
