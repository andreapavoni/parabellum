use crate::{
    components::{BuildingQueueItem, PageLayout, ResourceSlot, wrap_in_html},
    handlers::helpers::{CurrentUser, create_layout_data, village_queues_or_empty},
    http::AppState,
    pages::ResourcesPage,
    view_helpers::building_queue_to_views,
};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use dioxus::prelude::*;

/// GET /resources
pub async fn resources_page(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
    let queues = village_queues_or_empty(&state, user.village.id).await;
    let building_queue_views = building_queue_to_views(&queues.building);

    // Prepare resource slots data with processing state
    let resource_slots: Vec<ResourceSlot> = user
        .village
        .resource_fields()
        .into_iter()
        .map(|slot| {
            let in_queue = building_queue_views
                .iter()
                .find(|q| q.slot_id == slot.slot_id)
                .map(|q| q.is_processing);

            ResourceSlot {
                slot_id: slot.slot_id,
                building_name: slot.building.name.clone(),
                level: slot.building.level,
                in_queue,
            }
        })
        .collect();

    let building_queue: Vec<BuildingQueueItem> = building_queue_views
        .iter()
        .map(|item| BuildingQueueItem {
            slot_id: item.slot_id,
            building_name: item.building_name.to_string(),
            target_level: item.target_level,
            time_remaining: item.time_remaining.clone(),
            time_seconds: item.time_seconds,
            is_processing: item.is_processing,
        })
        .collect();

    // Prepare layout data
    let layout_data = create_layout_data(&user, "resources");

    // Render body with Dioxus SSR
    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data.clone(),
            ResourcesPage {
                village: layout_data.village.unwrap(),
                resource_slots,
                building_queue
            }
        }
    });

    // Wrap in full HTML document (includes app.js with tickers)
    let html = wrap_in_html(&body_content);
    Html(html)
}
