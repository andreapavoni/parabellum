use crate::{
    components::{
        BuildingQueueItem, LayoutData, PageLayout, ProductionInfo, QueueState, ResourceProduction,
        ResourceSlot, ResourcesPage, ResourcesPageData, TroopInfo, UserInfo, VillageCapacity,
        VillageHeaderData, VillageInfo, VillageResources, wrap_in_html,
    },
    handlers::{CurrentUser, village_queues_or_empty},
    http::AppState,
    view_helpers::{building_queue_to_views, unit_display_name},
};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use chrono::Utc;
use dioxus::prelude::*;

/// Render the resources page using Dioxus SSR
pub async fn resources_dioxus(
    State(state): State<AppState>,
    user: CurrentUser,
) -> impl IntoResponse {
    // Prepare resource slots data
    let resource_slots: Vec<ResourceSlot> = user
        .village
        .resource_fields()
        .into_iter()
        .map(|slot| ResourceSlot {
            slot_id: slot.slot_id,
            building_name: slot.building.name.clone(),
            level: slot.building.level,
            queue_state: None, // Will be populated below
        })
        .collect();

    // Get building queue
    let queues = village_queues_or_empty(&state, user.village.id).await;
    let building_queue_views = building_queue_to_views(&queues.building);

    // Update queue states in resource slots
    let mut resource_slots = resource_slots;
    for slot in &mut resource_slots {
        if let Some(queue_item) = building_queue_views
            .iter()
            .find(|q| q.slot_id == slot.slot_id)
        {
            slot.queue_state = Some(if queue_item.is_processing {
                QueueState::Active
            } else {
                QueueState::Pending
            });
        }
    }

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

    // Village info
    let village = VillageInfo {
        name: user.village.name.clone(),
        x: user.village.position.x,
        y: user.village.position.y,
    };

    // Prepare data for component
    let data = ResourcesPageData {
        village,
        resource_slots,
        production,
        troops,
        building_queue,
    };

    // Prepare layout data
    let layout_data = LayoutData {
        user: Some(UserInfo {
            username: user.player.username.clone(),
        }),
        village: Some(VillageHeaderData {
            resources: VillageResources {
                lumber: user.village.stored_resources().lumber(),
                clay: user.village.stored_resources().clay(),
                iron: user.village.stored_resources().iron(),
                crop: user.village.stored_resources().crop(),
            },
            production: ResourceProduction {
                lumber: user.village.production.effective.lumber,
                clay: user.village.production.effective.clay,
                iron: user.village.production.effective.iron,
                crop: user.village.production.effective.crop as u32,
            },
            capacity: VillageCapacity {
                warehouse: user.village.warehouse_capacity(),
                granary: user.village.granary_capacity(),
            },
            population: user.village.population,
        }),
        server_time: Utc::now().timestamp(),
        nav_active: "resources".to_string(),
    };

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
