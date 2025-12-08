use crate::{
    components::{
        BuildingQueueItem, PageLayout, ProductionInfo, ResourceSlot, ResourcesPage,
        ResourcesPageData, TroopInfo, wrap_in_html,
    },
    handlers::{CurrentUser, village_queues_or_empty},
    http::AppState,
    view_helpers::{building_queue_to_views, unit_display_name},
};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use dioxus::prelude::*;

use super::helpers::create_layout_data;

/// Render the resources page using Dioxus SSR
pub async fn resources(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
    // Get building queue
    let queues = village_queues_or_empty(&state, user.village.id).await;
    let building_queue_views = building_queue_to_views(&queues.building);

    // Prepare resource slots data with processing state
    let resource_slots: Vec<ResourceSlot> = user
        .village
        .resource_fields()
        .into_iter()
        .map(|slot| {
            let is_processing = building_queue_views
                .iter()
                .any(|q| q.slot_id == slot.slot_id && q.is_processing);

            ResourceSlot {
                slot_id: slot.slot_id,
                building_name: slot.building.name.clone(),
                level: slot.building.level,
                is_processing,
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

    // Get production info
    let production = ProductionInfo {
        lumber: user.village.production.effective.lumber,
        clay: user.village.production.effective.clay,
        iron: user.village.production.effective.iron,
        crop: user.village.production.effective.crop as u32,
    };

    // Get troops
    let troops: Vec<TroopInfo> = user
        .village
        .army()
        .map(|army| {
            let tribe_units = user.village.tribe.units();
            army.units()
                .iter()
                .enumerate()
                .filter_map(|(idx, quantity)| {
                    if *quantity == 0 {
                        return None;
                    }
                    let name = unit_display_name(&tribe_units[idx].name);
                    Some(TroopInfo {
                        name,
                        count: *quantity,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Prepare data for component
    let data = ResourcesPageData {
        village_name: user.village.name.clone(),
        village_x: user.village.position.x,
        village_y: user.village.position.y,
        resource_slots,
        production,
        troops,
        building_queue,
    };

    // Prepare layout data
    let layout_data = create_layout_data(&user, "resources");

    // Render body with Dioxus SSR
    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            ResourcesPage { data: data }
        }
    });

    // Wrap in full HTML document (includes app.js with tickers)
    let html = wrap_in_html(&body_content);
    Html(html)
}
