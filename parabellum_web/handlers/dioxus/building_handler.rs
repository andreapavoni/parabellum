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
use parabellum_game::models::buildings::{Building, get_building_data};
use parabellum_types::{army::UnitGroup, buildings::BuildingName, tribe::Tribe};

use crate::{
    components::{
        BuildingOption, EmptySlotPage, GenericBuildingPage, PageLayout, ResourceFieldPage,
        TrainingBuildingPage, TrainingQueueItem, UnitTrainingOption, wrap_in_html,
    },
    handlers::{
        CsrfForm, CurrentUser, HasCsrfToken, generate_csrf, village_movements_or_empty,
        village_queues_or_empty,
    },
    http::AppState,
    view_helpers::{building_queue_to_views, training_queue_to_views, unit_display_name},
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

    // If slot is empty, show EmptySlotPage
    let Some(slot_building) = slot_building else {
        let queue_views = building_queue_to_views(&queues.building);
        let has_queue_for_slot = queue_views.iter().any(|q| q.slot_id == slot_id);

        let (buildable_buildings, locked_buildings) = if has_queue_for_slot {
            (vec![], vec![])
        } else {
            building_options_for_slot(&user.village, slot_id, &queue_views, state.server_speed)
        };

        let body_content = dioxus_ssr::render_element(rsx! {
            PageLayout {
                data: layout_data,
                EmptySlotPage {
                    village: user.village.clone(),
                    slot_id: slot_id,
                    buildable_buildings: buildable_buildings,
                    locked_buildings: locked_buildings,
                    queue_full: queue_full,
                    has_queue_for_slot: has_queue_for_slot,
                    csrf_token: csrf_token,
                    flash_error: flash_error,
                }
            }
        });

        return Html(wrap_in_html(&body_content)).into_response();
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

    // Route to appropriate page component based on building type
    let body_content = match slot_building.building.name {
        BuildingName::Woodcutter
        | BuildingName::ClayPit
        | BuildingName::IronMine
        | BuildingName::Cropland => {
            // Resource fields - show production stats
            dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data,
                    ResourceFieldPage {
                        village: user.village.clone(),
                        slot_id: slot_id,
                        building_name: slot_building.building.name.clone(),
                        current_level: slot_building.building.level,
                        production_value: slot_building.building.value,
                        population: slot_building.building.population,
                        current_upkeep: slot_building.building.cost().upkeep,
                        next_level: next_level,
                        cost: cost.resources,
                        time_secs: time_secs,
                        next_upkeep: cost.upkeep,
                        queue_full: queue_full,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                    }
                }
            })
        }
        BuildingName::Barracks | BuildingName::GreatBarracks => {
            // Infantry training
            let training_units = training_options_for_group(
                &user.village,
                state.server_speed,
                Some(&slot_building),
                &[BuildingName::Barracks, BuildingName::GreatBarracks],
                UnitGroup::Infantry,
            );
            let training_queue = training_queue_for_slot(slot_id, &queues.training);

            dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data,
                    TrainingBuildingPage {
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
                        training_units: training_units,
                        training_queue: training_queue,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                    }
                }
            })
        }
        BuildingName::Stable | BuildingName::GreatStable => {
            // Cavalry training
            let training_units = training_options_for_group(
                &user.village,
                state.server_speed,
                Some(&slot_building),
                &[BuildingName::Stable, BuildingName::GreatStable],
                UnitGroup::Cavalry,
            );
            let training_queue = training_queue_for_slot(slot_id, &queues.training);

            dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data,
                    TrainingBuildingPage {
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
                        training_units: training_units,
                        training_queue: training_queue,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                    }
                }
            })
        }
        BuildingName::Workshop | BuildingName::GreatWorkshop => {
            // Siege training
            let training_units = training_options_for_group(
                &user.village,
                state.server_speed,
                Some(&slot_building),
                &[BuildingName::Workshop, BuildingName::GreatWorkshop],
                UnitGroup::Siege,
            );
            let training_queue = training_queue_for_slot(slot_id, &queues.training);

            dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data,
                    TrainingBuildingPage {
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
                        training_units: training_units,
                        training_queue: training_queue,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                    }
                }
            })
        }
        _ => {
            // Generic buildings - just upgrade block
            // TODO: Add specific pages for Academy, Smithy, RallyPoint
            dioxus_ssr::render_element(rsx! {
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
            })
        }
    };

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

/// Calculate building options for an empty slot
fn building_options_for_slot(
    village: &parabellum_game::models::village::Village,
    slot_id: u8,
    queue_views: &[crate::templates::BuildingQueueItemView],
    server_speed: i8,
) -> (Vec<BuildingOption>, Vec<BuildingOption>) {
    let mut buildable = Vec::new();
    let mut locked = Vec::new();
    let main_building_level = village.main_building_level();

    for name in village.candidate_buildings_for_slot(slot_id) {
        if building_blocked_by_queue(&name, queue_views) {
            continue;
        }

        let building = Building::new(name.clone(), server_speed);
        let validation_ok = village.validate_building_construction(&building).is_ok();
        let missing_requirements = missing_requirements_for_building(village, &name);

        if !validation_ok && missing_requirements.is_empty() {
            continue;
        }

        let cost = building.cost();
        let time_secs = building.calculate_build_time_secs(&server_speed, &main_building_level);

        let option = BuildingOption {
            name,
            cost: cost.resources,
            time_secs,
            missing_requirements,
        };

        if validation_ok {
            buildable.push(option);
        } else {
            locked.push(option);
        }
    }

    (buildable, locked)
}

fn building_blocked_by_queue(
    name: &BuildingName,
    queue: &[crate::templates::BuildingQueueItemView],
) -> bool {
    if queue.is_empty() {
        return false;
    }

    let Ok(candidate_data) = get_building_data(name) else {
        return false;
    };

    queue.iter().any(|job| {
        let queued_name = &job.building_name;
        (!candidate_data.rules.allow_multiple && queued_name == name)
            || candidate_data
                .rules
                .conflicts
                .iter()
                .any(|conflict| conflict.0 == *queued_name)
            || conflicts_with_queued(name, queued_name)
    })
}

fn conflicts_with_queued(candidate: &BuildingName, queued: &BuildingName) -> bool {
    match get_building_data(queued) {
        Ok(data) => {
            (!data.rules.allow_multiple && queued == candidate)
                || data
                    .rules
                    .conflicts
                    .iter()
                    .any(|conflict| conflict.0 == *candidate)
        }
        Err(_) => false,
    }
}

fn missing_requirements_for_building(
    village: &parabellum_game::models::village::Village,
    name: &BuildingName,
) -> Vec<(BuildingName, u8)> {
    let Ok(data) = get_building_data(name) else {
        return vec![];
    };

    data.rules
        .requirements
        .iter()
        .filter_map(|req| {
            let level = village
                .buildings()
                .iter()
                .find(|vb| vb.building.name == req.0)
                .map(|vb| vb.building.level)
                .unwrap_or(0);

            if level >= req.1 {
                None
            } else {
                Some((req.0.clone(), req.1))
            }
        })
        .collect()
}

/// Calculate training options for a unit group
fn training_options_for_group(
    village: &parabellum_game::models::village::Village,
    server_speed: i8,
    building: Option<&parabellum_game::models::village::VillageBuilding>,
    expected_buildings: &[BuildingName],
    group: UnitGroup,
) -> Vec<UnitTrainingOption> {
    let Some(slot) = building else {
        return vec![];
    };

    if !expected_buildings.contains(&slot.building.name) {
        return vec![];
    }

    let training_multiplier = slot.building.value as f64 / 1000.0;
    let available_units = village.available_units_for_training(group);
    let tribe = village.tribe.clone();

    available_units
        .into_iter()
        .filter_map(|unit| {
            let unit_idx = tribe.get_unit_idx_by_name(&unit.name)? as u8;
            let base_time_per_unit = unit.cost.time as f64 / server_speed as f64;
            let time_per_unit = (base_time_per_unit * training_multiplier).floor().max(1.0) as u32;

            Some(UnitTrainingOption {
                unit_idx,
                name: unit_display_name(&unit.name),
                cost: unit.cost.resources.clone(),
                upkeep: unit.cost.upkeep,
                time_secs: time_per_unit,
            })
        })
        .collect()
}

/// Get training queue for a specific slot
fn training_queue_for_slot(
    slot_id: u8,
    queue: &[parabellum_app::cqrs::queries::TrainingQueueItem],
) -> Vec<TrainingQueueItem> {
    let queue_views = training_queue_to_views(queue);
    queue_views
        .into_iter()
        .filter(|item| item.slot_id == slot_id)
        .map(|item| TrainingQueueItem {
            quantity: item.quantity as u32,
            unit_name: item.unit_name,
            time_per_unit: item.time_per_unit as u32,
            time_remaining_secs: item.time_seconds,
        })
        .collect()
}
