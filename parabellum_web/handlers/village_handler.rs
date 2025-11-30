use axum::{extract::State, response::IntoResponse};
use tracing::error;

use crate::{
    handlers::{CurrentUser, load_building_queue, render_template},
    http::AppState,
    templates::{ResourceField, ResourcesTemplate, VillageTemplate},
    view_helpers::{building_queue_to_views, resource_css_class, server_time_context},
};

pub async fn village(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
    let building_queue = match load_building_queue(&state, user.village.id).await {
        Ok(items) => building_queue_to_views(&items),
        Err(err) => {
            error!(
                error = ?err,
                village_id = user.village.id,
                "Unable to load building queue"
            );
            Vec::new()
        }
    };

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

    let building_queue = match load_building_queue(&state, user.village.id).await {
        Ok(items) => building_queue_to_views(&items),
        Err(err) => {
            error!(
                error = ?err,
                village_id = user.village.id,
                "Unable to load building queue"
            );
            Vec::new()
        }
    };

    let template = ResourcesTemplate {
        current_user: Some(user),
        nav_active: "resources",
        resource_slots,
        building_queue,
        server_time: server_time_context(),
    };
    render_template(template, None).into_response()
}
