use crate::{
    handlers::{CurrentUser, render_template, village_queues_or_empty},
    http::AppState,
    templates::{
        ResourceField, ResourcesTemplate, TemplateLayout, TroopCountView, VillageTemplate,
    },
    view_helpers::{building_queue_to_views, resource_css_class, unit_display_name},
};
use axum::{extract::State, response::IntoResponse};
use std::collections::HashMap;

pub async fn village(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
    let queues = village_queues_or_empty(&state, user.village.id).await;
    let building_queue = building_queue_to_views(&queues.building);

    let slot_buildings = user
        .village
        .buildings()
        .iter()
        .map(|vb| (vb.slot_id, vb.clone()))
        .collect::<HashMap<_, _>>();

    let template = VillageTemplate {
        layout: TemplateLayout::new(Some(user), "village"),
        building_queue,
        slot_buildings,
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

    let queues = village_queues_or_empty(&state, user.village.id).await;
    let building_queue = building_queue_to_views(&queues.building);

    let home_troops = user
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
                    Some(TroopCountView {
                        name,
                        count: *quantity,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let template = ResourcesTemplate {
        layout: TemplateLayout::new(Some(user), "resources"),
        resource_slots,
        building_queue,
        home_troops,
    };
    render_template(template, None).into_response()
}
