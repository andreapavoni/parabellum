use crate::{
    components::{
        BuildingQueueItem, BuildingSlot, PageLayout, VillageListItem, VillagePage, VillagePageData,
        wrap_in_html,
    },
    handlers::{CurrentUser, village_queues_or_empty},
    http::AppState,
    view_helpers::building_queue_to_views,
};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use dioxus::prelude::*;

use super::helpers::create_layout_data;

/// Render the village center page using Dioxus SSR
pub async fn village(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
    let queues = village_queues_or_empty(&state, user.village.id).await;
    let building_queue_views = building_queue_to_views(&queues.building);

    let building_slots: Vec<BuildingSlot> = user
        .village
        .buildings()
        .iter()
        .map(|vb| {
            let in_queue = building_queue_views
                .iter()
                .find(|q| q.slot_id == vb.slot_id)
                .map(|q| q.is_processing);

            BuildingSlot {
                slot_id: vb.slot_id,
                building_name: Some(vb.building.name.clone()),
                level: vb.building.level,
                in_queue,
            }
        })
        .collect();

    let building_queue = building_queue_views
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

    let villages = user
        .villages
        .iter()
        .map(|v| VillageListItem {
            id: v.id as i64,
            name: v.name.clone(),
            x: v.position.x,
            y: v.position.y,
            is_current: v.id == user.village.id,
        })
        .collect();

    let data = VillagePageData {
        village_name: user.village.name.clone(),
        village_x: user.village.position.x,
        village_y: user.village.position.y,
        building_slots,
        building_queue,
        villages,
    };

    let layout_data = create_layout_data(&user, "village");

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            VillagePage { data: data }
        }
    });

    Html(wrap_in_html(&body_content))
}
