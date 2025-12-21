use crate::{
    components::{BuildingQueueItem, BuildingSlot, PageLayout, VillageListItem, wrap_in_html},
    handlers::helpers::{
        CsrfForm, CurrentUser, HasCsrfToken, generate_csrf, village_queues_or_empty,
    },
    http::AppState,
    pages::VillagePage,
    view_helpers::building_queue_to_views,
};
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect},
};
use axum_extra::extract::{SignedCookieJar, cookie::Cookie};
use dioxus::prelude::*;
use serde::Deserialize;

use super::helpers::create_layout_data;

/// Render the village center page using Dioxus SSR
pub async fn village_page(
    State(state): State<AppState>,
    user: CurrentUser,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    let (jar, csrf_token) = generate_csrf(jar);
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
            id: v.id,
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
                villages,
                csrf_token
            }
        }
    });

    (jar, Html(wrap_in_html(&body_content))).into_response()
}

#[derive(Debug, Deserialize)]
pub struct SwitchVillageForm {
    pub csrf_token: String,
}

impl HasCsrfToken for SwitchVillageForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

/// POST /village/switch/{id} - Switch current village
pub async fn switch_village(
    Path(id): Path<u32>,
    user: CurrentUser,
    CsrfForm { jar, form: _ }: CsrfForm<SwitchVillageForm>,
) -> impl IntoResponse {
    if !user.villages.iter().any(|v| v.id == id) {
        tracing::warn!(
            "User {} attempted to switch to invalid village {}",
            user.player.id,
            id
        );
        return Redirect::to("/village").into_response();
    }

    let cookie = Cookie::build(Cookie::new("current_village_id", id.to_string()))
        .path("/")
        .build();
    let jar = jar.add(cookie);
    (jar, Redirect::to("/village")).into_response()
}
