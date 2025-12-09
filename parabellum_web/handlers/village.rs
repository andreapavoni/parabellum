use crate::{
    components::{BuildingQueueItem, BuildingSlot, PageLayout, VillageListItem, wrap_in_html},
    handlers::helpers::{CurrentUser, village_queues_or_empty},
    http::AppState,
    pages::VillagePage,
    view_helpers::building_queue_to_views,
};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use dioxus::prelude::*;

use super::helpers::create_layout_data;

/// Render the village center page using Dioxus SSR
pub async fn village_page(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
    let queues = village_queues_or_empty(&state, user.village.id).await;
    let building_queue_views = building_queue_to_views(&queues.building);

    // Create building slots for ALL slots (19-40), including empty ones
    let building_slots: Vec<BuildingSlot> = (19..=40)
        .map(|slot_id| {
            let building = user
                .village
                .buildings()
                .iter()
                .find(|vb| vb.slot_id == slot_id);

            let in_queue = building_queue_views
                .iter()
                .find(|q| q.slot_id == slot_id)
                .map(|q| q.is_processing);

            if let Some(vb) = building {
                BuildingSlot {
                    slot_id,
                    building_name: Some(vb.building.name.clone()),
                    level: vb.building.level,
                    in_queue,
                }
            } else {
                BuildingSlot {
                    slot_id,
                    building_name: None,
                    level: 0,
                    in_queue,
                }
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

    let layout_data = create_layout_data(&user, "village");

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data.clone(),
            VillagePage {
                village: layout_data.village.unwrap(),
                building_slots,
                building_queue,
                villages
            }
        }
    });

    Html(wrap_in_html(&body_content))
}
