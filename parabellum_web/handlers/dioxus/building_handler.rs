use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::SignedCookieJar;
use dioxus::prelude::*;
use serde::Deserialize;

use parabellum_app::{
    command_handlers::{AddBuildingCommandHandler, UpgradeBuildingCommandHandler},
    cqrs::{
        commands::{AddBuilding, UpgradeBuilding},
        queries::{VillageQueues, VillageTroopMovements},
    },
};
use parabellum_types::{buildings::BuildingName, tribe::Tribe};

use crate::{
    components::{GenericBuildingPage, PageLayout, wrap_in_html},
    handlers::{
        CsrfForm, CurrentUser, HasCsrfToken, generate_csrf, village_movements_or_empty,
        village_queues_or_empty,
    },
    http::AppState,
};

use super::helpers::create_layout_data;

pub(super) const MAX_SLOT_ID: u8 = 40;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildAction {
    Build,
    Upgrade,
}

#[derive(Debug, Deserialize)]
pub struct BuildActionForm {
    pub slot_id: u8,
    pub action: BuildAction,
    pub building_name: Option<BuildingName>,
    pub csrf_token: String,
}

impl HasCsrfToken for BuildActionForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

/// GET /dioxus/build/{slot_id} - View building page
pub async fn building(
    State(state): State<AppState>,
    Path(slot_id): Path<u8>,
    user: CurrentUser,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    // Validate slot_id
    if !(1..=MAX_SLOT_ID).contains(&slot_id) {
        return Redirect::to("/dioxus/village").into_response();
    }

    let queues = village_queues_or_empty(&state, user.village.id).await;
    let movements = rally_point_movements_for_slot(&state, &user, slot_id).await;

    let (jar, csrf_token) = generate_csrf(jar);
    let flash_error = None; // TODO: Get from flash messages when needed

    let response = render_building_page(
        &state,
        &user,
        slot_id,
        csrf_token,
        flash_error,
        queues,
        movements,
    );

    (jar, response).into_response()
}

/// POST /dioxus/build/{slot_id} - Execute build/upgrade action
pub async fn build_action(
    State(state): State<AppState>,
    Path(slot_id): Path<u8>,
    user: CurrentUser,
    CsrfForm { jar, form }: CsrfForm<BuildActionForm>,
) -> Response {
    // Validate slot_id
    if !(1..=MAX_SLOT_ID).contains(&slot_id) {
        return Redirect::to("/dioxus/village").into_response();
    }

    // Validate form slot_id matches path
    if slot_id != form.slot_id {
        return render_with_error(
            &state,
            jar,
            user,
            slot_id,
            "Slot mismatch, please retry.".to_string(),
        )
        .await;
    }

    let result = match form.action {
        BuildAction::Build => {
            let building_name = match form.building_name.clone() {
                Some(name) => name,
                None => {
                    return render_with_error(
                        &state,
                        jar,
                        user,
                        slot_id,
                        "Missing building name.".to_string(),
                    )
                    .await;
                }
            };

            state
                .app_bus
                .execute(
                    AddBuilding {
                        player_id: user.player.id,
                        village_id: user.village.id,
                        slot_id: form.slot_id,
                        name: building_name,
                    },
                    AddBuildingCommandHandler::new(),
                )
                .await
        }
        BuildAction::Upgrade => {
            state
                .app_bus
                .execute(
                    UpgradeBuilding {
                        player_id: user.player.id,
                        village_id: user.village.id,
                        slot_id: form.slot_id,
                    },
                    UpgradeBuildingCommandHandler::new(),
                )
                .await
        }
    };

    match result {
        Ok(()) => Redirect::to(&format!("/dioxus/build/{slot_id}")).into_response(),
        Err(err) => render_with_error(&state, jar, user, slot_id, err.to_string()).await,
    }
}

/// Render building page based on slot contents
fn render_building_page(
    state: &AppState,
    user: &CurrentUser,
    slot_id: u8,
    csrf_token: String,
    flash_error: Option<String>,
    queues: VillageQueues,
    _movements: VillageTroopMovements,
) -> Response {
    let layout_data = create_layout_data(
        user,
        if slot_id <= 18 {
            "resources"
        } else {
            "village"
        },
    );

    let slot_building = user.village.get_building_by_slot_id(slot_id);

    // Calculate queue state
    let building_queue_capacity: usize = if matches!(user.village.tribe, Tribe::Roman) {
        3
    } else {
        2
    };
    let queue_full = queues.building.len() >= building_queue_capacity;

    // If slot is empty, show EmptySlotPage (TODO)
    let Some(slot_building) = slot_building else {
        let body_content = "<div class='p-4'>Empty slot - Coming soon</div>";
        return Html(wrap_in_html(body_content)).into_response();
    };

    // Calculate upgrade info
    let main_building_level = user.village.main_building_level();
    let next_level = slot_building.building.level.saturating_add(1);

    let upgraded = match slot_building
        .building
        .clone()
        .at_level(next_level, state.server_speed)
    {
        Ok(b) => b,
        Err(_) => {
            // Max level reached
            let body_content = format!(
                "<div class='p-4'>{} at max level ({})</div>",
                slot_building.building.name, slot_building.building.level
            );
            return Html(wrap_in_html(&body_content)).into_response();
        }
    };

    let cost = upgraded.cost();
    let time_secs = upgraded.calculate_build_time_secs(&state.server_speed, &main_building_level);

    // For now, render GenericBuildingPage for all buildings
    // TODO: Add specific pages for Barracks, Stable, Workshop, Academy, Smithy, RallyPoint
    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            GenericBuildingPage {
                village: user.village.clone(),
                slot_id: slot_id,
                building_name: slot_building.building.name.clone(),
                current_level: slot_building.building.level,
                next_level: next_level,
                cost: cost.resources,
                time_secs: time_secs,
                current_upkeep: slot_building.building.cost().upkeep,
                next_upkeep: cost.upkeep,
                queue_full: queue_full,
                csrf_token: csrf_token,
                flash_error: flash_error,
            }
        }
    });

    Html(wrap_in_html(&body_content)).into_response()
}

/// Helper to fetch rally point movements for a slot
async fn rally_point_movements_for_slot(
    state: &AppState,
    user: &CurrentUser,
    slot_id: u8,
) -> VillageTroopMovements {
    let slot_building = user.village.get_building_by_slot_id(slot_id);
    match slot_building {
        Some(building) if building.building.name == BuildingName::RallyPoint => {
            village_movements_or_empty(state, user.village.id).await
        }
        _ => VillageTroopMovements {
            incoming: vec![],
            outgoing: vec![],
        },
    }
}

/// Render page with error message
async fn render_with_error(
    state: &AppState,
    jar: SignedCookieJar,
    user: CurrentUser,
    slot_id: u8,
    error: String,
) -> Response {
    let queues = village_queues_or_empty(state, user.village.id).await;
    let movements = rally_point_movements_for_slot(state, &user, slot_id).await;
    let (_jar, csrf_token) = generate_csrf(jar);

    let response = render_building_page(
        state,
        &user,
        slot_id,
        csrf_token,
        Some(error),
        queues,
        movements,
    );

    response
}
