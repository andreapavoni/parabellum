use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::SignedCookieJar;
use serde::Deserialize;

use parabellum_app::{
    command_handlers::{AddBuildingCommandHandler, UpgradeBuildingCommandHandler},
    cqrs::commands::{AddBuilding, UpgradeBuilding},
    cqrs::queries::BuildingQueueItem,
};
use parabellum_game::models::{buildings::Building, village::VillageBuilding};
use parabellum_types::buildings::BuildingName;

use crate::{
    handlers::{
        CsrfForm, CurrentUser, HasCsrfToken, building_queue_or_empty, generate_csrf,
        render_template,
    },
    http::AppState,
    templates::{BuildingOption, BuildingQueueItemView, BuildingTemplate, BuildingUpgradeInfo},
    view_helpers::{building_queue_to_views, format_duration, server_time_context},
};

#[derive(Debug, Deserialize)]
pub struct BuildParams {
    #[serde(rename = "s", default)]
    slot_id: Option<u8>,
}

const MAX_SLOT_ID: u8 = 40;

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

pub async fn building(
    State(state): State<AppState>,
    Query(params): Query<BuildParams>,
    user: CurrentUser,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    let slot_id = match params.slot_id {
        Some(slot) if (1..=MAX_SLOT_ID).contains(&slot) => slot,
        _ => return Redirect::to("/village").into_response(),
    };

    let queue_items = building_queue_or_empty(&state, user.village.id).await;

    let (jar, csrf_token) = generate_csrf(jar);
    let template = build_template(&state, &user, slot_id, csrf_token, None, queue_items);
    (jar, render_template(template, None)).into_response()
}

pub async fn build_action(
    State(state): State<AppState>,
    Query(params): Query<BuildParams>,
    user: CurrentUser,
    CsrfForm { jar, inner: form }: CsrfForm<BuildActionForm>,
) -> Response {
    let slot_id = match params.slot_id {
        Some(slot) if (1..=MAX_SLOT_ID).contains(&slot) => slot,
        _ => return Redirect::to("/village").into_response(),
    };

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
        Ok(()) => Redirect::to(&format!("/build?s={slot_id}")).into_response(),
        Err(err) => render_with_error(&state, jar, user, slot_id, err.to_string()).await,
    }
}

fn build_option(name: BuildingName, server_speed: i8, main_building_level: u8) -> BuildingOption {
    let building = Building::new(name.clone(), server_speed);
    let cost = building.cost();
    let time_secs = building.calculate_build_time_secs(&server_speed, &main_building_level);
    let key = serde_json::to_value(&name)
        .expect("BuildingName should serialize to a string")
        .as_str()
        .expect("BuildingName should serialize to a string")
        .to_string();

    BuildingOption {
        name,
        key,
        cost: cost.resources.into(),
        upkeep: cost.upkeep,
        time_formatted: format_duration(time_secs),
    }
}

fn building_upgrade_info(
    slot_building: &VillageBuilding,
    server_speed: i8,
    main_building_level: u8,
) -> Option<BuildingUpgradeInfo> {
    let current_upkeep = slot_building.building.cost().upkeep;
    let next_level = slot_building.building.level.saturating_add(1);
    let upgraded = slot_building
        .building
        .clone()
        .at_level(next_level, server_speed)
        .ok()?;
    let cost = upgraded.cost();
    let time_secs = upgraded.calculate_build_time_secs(&server_speed, &main_building_level);

    Some(BuildingUpgradeInfo {
        next_level,
        cost: cost.resources.into(),
        current_upkeep,
        upkeep: cost.upkeep,
        time_formatted: format_duration(time_secs),
    })
}

async fn render_with_error(
    state: &AppState,
    jar: SignedCookieJar,
    user: CurrentUser,
    slot_id: u8,
    error: String,
) -> Response {
    let queue_items = building_queue_or_empty(state, user.village.id).await;
    let (jar, csrf_token) = generate_csrf(jar);
    let template = build_template(state, &user, slot_id, csrf_token, Some(error), queue_items);
    (
        jar,
        render_template(template, Some(StatusCode::BAD_REQUEST)),
    )
        .into_response()
}

fn build_template(
    state: &AppState,
    user: &CurrentUser,
    slot_id: u8,
    csrf_token: String,
    flash_error: Option<String>,
    queue_items: Vec<BuildingQueueItem>,
) -> BuildingTemplate {
    let slot_building = user.village.get_building_by_slot_id(slot_id);
    let main_building_level = user.village.main_building_level();
    let queue_view = building_queue_to_views(&queue_items);
    let queue_for_slot = queue_view
        .iter()
        .filter(|item| item.slot_id == slot_id)
        .cloned()
        .collect::<Vec<_>>();
    let current_construction = queue_for_slot.first().cloned();

    let effective_building = virtual_building_after_queue(
        slot_building.clone(),
        queue_for_slot.last(),
        state.server_speed,
    );

    let upgrade = effective_building
        .as_ref()
        .and_then(|slot| building_upgrade_info(slot, state.server_speed, main_building_level));
    let current_upkeep = effective_building
        .as_ref()
        .map(|slot| slot.building.cost().upkeep);

    let available_buildings = if slot_building.is_none() && queue_for_slot.is_empty() {
        user.village
            .available_buildings_for_slot(slot_id)
            .into_iter()
            .map(|name| build_option(name, state.server_speed, main_building_level))
            .collect::<Vec<BuildingOption>>()
    } else {
        vec![]
    };

    let nav_active = if slot_id <= 18 {
        "resources"
    } else {
        "village"
    };
    let available_resources = user.village.stored_resources().into();

    BuildingTemplate {
        current_user: Some(user.clone()),
        nav_active,
        slot_id,
        slot_building,
        available_buildings,
        upgrade,
        current_upkeep,
        csrf_token,
        flash_error,
        building_queue: queue_view,
        current_construction,
        queue_for_slot,
        available_resources,
        server_time: server_time_context(),
    }
}

fn virtual_building_after_queue(
    slot_building: Option<VillageBuilding>,
    last_queue_item: Option<&BuildingQueueItemView>,
    server_speed: i8,
) -> Option<VillageBuilding> {
    match (slot_building, last_queue_item) {
        (Some(building), Some(queue)) => {
            let upgraded = building
                .building
                .clone()
                .at_level(queue.target_level, server_speed)
                .ok()?;
            Some(VillageBuilding {
                slot_id: building.slot_id,
                building: upgraded,
            })
        }
        (None, Some(queue)) => {
            let building = Building::new(queue.building_name.clone(), server_speed)
                .at_level(queue.target_level, server_speed)
                .ok()?;
            Some(VillageBuilding {
                slot_id: queue.slot_id,
                building,
            })
        }
        (Some(building), None) => Some(building),
        (None, None) => None,
    }
}
