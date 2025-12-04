use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::SignedCookieJar;
use serde::Deserialize;

use parabellum_app::{
    command_handlers::{AddBuildingCommandHandler, UpgradeBuildingCommandHandler},
    cqrs::{
        commands::{AddBuilding, UpgradeBuilding},
        queries::{BuildingQueueItem, TrainingQueueItem},
    },
};
use parabellum_game::models::{
    buildings::{Building, get_building_data},
    village::{Village, VillageBuilding},
};
use parabellum_types::{army::UnitGroup, buildings::BuildingName};

use crate::{
    handlers::{
        CsrfForm, CurrentUser, HasCsrfToken, building_queue_or_empty, generate_csrf,
        render_template, training_queue_or_empty,
    },
    http::AppState,
    templates::{
        BuildingOption, BuildingQueueItemView, BuildingRequirementView, BuildingTemplate,
        BuildingUpgradeInfo, UnitTrainingOption,
    },
    view_helpers::{
        building_queue_to_views, format_duration, server_time, training_queue_to_views,
        unit_display_name,
    },
};

#[derive(Debug, Deserialize)]
pub struct BuildParams {
    #[serde(rename = "s", default)]
    pub slot_id: Option<u8>,
}

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
    let training_queue_items = training_queue_or_empty(&state, user.village.id).await;

    let (jar, csrf_token) = generate_csrf(jar);
    let template = build_template(
        &state,
        &user,
        slot_id,
        csrf_token,
        None,
        queue_items,
        training_queue_items,
    );
    (jar, render_template(template, None)).into_response()
}

pub async fn build_action(
    State(state): State<AppState>,
    Query(params): Query<BuildParams>,
    user: CurrentUser,
    CsrfForm { jar, form }: CsrfForm<BuildActionForm>,
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

fn build_option(
    name: BuildingName,
    server_speed: i8,
    main_building_level: u8,
    missing_requirements: Vec<BuildingRequirementView>,
    can_start: bool,
) -> BuildingOption {
    let building = Building::new(name.clone(), server_speed);
    let cost = building.cost();
    let time_secs = building.calculate_build_time_secs(&server_speed, &main_building_level);

    BuildingOption {
        name,
        cost: cost.resources.into(),
        upkeep: cost.upkeep,
        time_formatted: format_duration(time_secs),
        missing_requirements,
        can_start,
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

pub(crate) async fn render_with_error(
    state: &AppState,
    jar: SignedCookieJar,
    user: CurrentUser,
    slot_id: u8,
    error: String,
) -> Response {
    let queue_items = building_queue_or_empty(state, user.village.id).await;
    let training_queue_items = training_queue_or_empty(state, user.village.id).await;
    let (jar, csrf_token) = generate_csrf(jar);
    let template = build_template(
        state,
        &user,
        slot_id,
        csrf_token,
        Some(error),
        queue_items,
        training_queue_items,
    );
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
    training_queue: Vec<TrainingQueueItem>,
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

    let training_queue_view = training_queue_to_views(&training_queue);
    let training_queue_for_slot = training_queue_view
        .iter()
        .filter(|item| item.slot_id == slot_id)
        .cloned()
        .collect::<Vec<_>>();

    let available_buildings = if slot_building.is_none() && queue_for_slot.is_empty() {
        user
            .village
            .candidate_buildings_for_slot(slot_id)
            .into_iter()
            .filter_map(|name| {
                if building_blocked_by_queue(&name, &queue_view) {
                    return None;
                }

                let building = Building::new(name.clone(), state.server_speed);
                let validation_ok = user
                    .village
                    .validate_building_construction(&building)
                    .is_ok();
                let missing_requirements =
                    missing_requirements_for_building(&user.village, &name);
                let should_show = validation_ok || !missing_requirements.is_empty();

                if !should_show {
                    return None;
                }

                Some(build_option(
                    name,
                    state.server_speed,
                    main_building_level,
                    missing_requirements,
                    validation_ok,
                ))
            })
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
    let barracks_units = training_options_for_group(
        &user.village,
        state.server_speed,
        effective_building.as_ref(),
        BuildingName::Barracks,
        UnitGroup::Infantry,
    );
    let stable_units = training_options_for_group(
        &user.village,
        state.server_speed,
        effective_building.as_ref(),
        BuildingName::Stable,
        UnitGroup::Cavalry,
    );
    let workshop_units = training_options_for_group(
        &user.village,
        state.server_speed,
        effective_building.as_ref(),
        BuildingName::Workshop,
        UnitGroup::Siege,
    );

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
        current_construction,
        available_resources,
        server_time: server_time(),
        barracks_units,
        stable_units,
        workshop_units,
        training_queue_for_slot,
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

fn training_options_for_group(
    village: &Village,
    server_speed: i8,
    building: Option<&VillageBuilding>,
    expected_building: BuildingName,
    group: UnitGroup,
) -> Vec<UnitTrainingOption> {
    let Some(slot) = building else {
        return vec![];
    };

    if slot.building.name != expected_building {
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
                cost: unit.cost.resources.clone().into(),
                upkeep: unit.cost.upkeep,
                time_formatted: format_duration(time_per_unit),
            })
        })
        .collect()
}

fn building_blocked_by_queue(
    name: &BuildingName,
    queue: &[BuildingQueueItemView],
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
    village: &Village,
    name: &BuildingName,
) -> Vec<BuildingRequirementView> {
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
                Some(BuildingRequirementView {
                    name: req.0.clone(),
                    level: req.1,
                })
            }
        })
        .collect()
}
