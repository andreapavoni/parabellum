use crate::{
    components::{
        BuildingQueueItem, ProductionInfo, QueueState, ResourceSlot, ResourcesPage,
        ResourcesPageData, TroopInfo, VillageInfo,
    },
    handlers::{CurrentUser, village_queues_or_empty},
    http::AppState,
    view_helpers::{building_queue_to_views, resource_css_class, unit_display_name},
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
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
        .map(|slot| {
            let css_class = resource_css_class(Some(&slot)).to_string();
            let building_name = slot.building.name.clone();
            let level = slot.building.level;
            ResourceSlot {
                slot_id: slot.slot_id,
                building_name,
                level,
                css_class,
                queue_state: None, // Will be populated below
            }
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

    // Render with Dioxus SSR
    let html = dioxus_ssr::render_element(rsx! {
        ResourcesPage { data: data }
    });

    // Wrap in a full HTML layout
    // For now, we'll use a simple layout. Later you might want to create a layout component
    let full_html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Resources – PARABELLUM</title>
    <link rel="stylesheet" href="/assets/tailwind.css">
    <link rel="stylesheet" href="/assets/index.css">
</head>
<body class="flex flex-col min-h-screen">
    <main class="flex-grow container mx-auto">
        {}
    </main>
    <script src="/assets/index.js" type="application/javascript"></script>
</body>
</html>"#,
        html
    );

    Html(full_html)
}

/// JSON API endpoint for resources page data
pub async fn resources_api(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Response, StatusCode> {
    // Prepare the same data structure
    let resource_slots: Vec<ResourceSlot> = user
        .village
        .resource_fields()
        .into_iter()
        .map(|slot| {
            let css_class = resource_css_class(Some(&slot)).to_string();
            let building_name = slot.building.name.clone();
            let level = slot.building.level;
            ResourceSlot {
                slot_id: slot.slot_id,
                building_name,
                level,
                css_class,
                queue_state: None,
            }
        })
        .collect();

    let queues = village_queues_or_empty(&state, user.village.id).await;
    let building_queue_views = building_queue_to_views(&queues.building);

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

    let production = ProductionInfo {
        lumber: user.village.production.effective.lumber,
        clay: user.village.production.effective.clay,
        iron: user.village.production.effective.iron,
        crop: user.village.production.effective.crop as u32,
    };

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

    let village = VillageInfo {
        name: user.village.name.clone(),
        x: user.village.position.x,
        y: user.village.position.y,
    };

    let data = ResourcesPageData {
        village,
        resource_slots,
        production,
        troops,
        building_queue,
    };

    Ok(axum::Json(data).into_response())
}
