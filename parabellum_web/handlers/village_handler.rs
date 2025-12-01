use crate::{
    handlers::{CurrentUser, building_queue_or_empty, render_template},
    http::AppState,
    templates::{ResourceField, ResourcesTemplate, VillageTemplate},
    view_helpers::{building_queue_to_views, resource_css_class, server_time_context},
};
use axum::{extract::State, response::IntoResponse};

pub async fn village(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
    let building_queue =
        building_queue_to_views(&building_queue_or_empty(&state, user.village.id).await);

    let template = VillageTemplate {
        current_user: Some(user),
        nav_active: "village",
        building_queue,
        server_time: server_time_context(),
    };
    render_template(template, None).into_response()
}

pub async fn resources(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
    let resource_slots = user
        .village
        .resource_fields()
        .into_iter()
        .map(|slot| {
            let class = resource_css_class(Some(&slot));
            let name = slot.building.name.clone();
            let level = slot.building.level;
            ResourceField { class, name, level }
        })
        .collect();

    let building_queue =
        building_queue_to_views(&building_queue_or_empty(&state, user.village.id).await);

    let template = ResourcesTemplate {
        current_user: Some(user),
        nav_active: "resources",
        resource_slots,
        building_queue,
        server_time: server_time_context(),
    };
    render_template(template, None).into_response()
}
