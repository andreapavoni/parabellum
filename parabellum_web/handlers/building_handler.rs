use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;

use parabellum_game::models::{buildings::Building, village::VillageBuilding};
use parabellum_types::buildings::BuildingName;

use crate::{
    handlers::{CurrentUser, render_template},
    http::AppState,
    templates::{BuildingOption, BuildingTemplate, BuildingUpgradeInfo, ResourceCostView},
    view_helpers::format_duration,
};

#[derive(Debug, Deserialize)]
pub struct BuildParams {
    #[serde(rename = "s", default)]
    slot_id: Option<u8>,
}

const MAX_SLOT_ID: u8 = 40;

pub async fn building(
    State(state): State<AppState>,
    Query(params): Query<BuildParams>,
    user: CurrentUser,
) -> impl IntoResponse {
    let slot_id = match params.slot_id {
        Some(slot) if (1..=MAX_SLOT_ID).contains(&slot) => slot,
        _ => return Redirect::to("/village").into_response(),
    };

    let slot_building = user.village.get_building_by_slot_id(slot_id);
    let main_building_level = user.village.main_building_level();
    let current_upkeep = slot_building
        .as_ref()
        .map(|slot| slot.building.cost().upkeep);
    let upgrade = slot_building
        .as_ref()
        .and_then(|slot| building_upgrade_info(slot, state.server_speed, main_building_level));

    let available_buildings = if slot_building.is_none() {
        user.village
            .available_buildings_for_slot(slot_id)
            .into_iter()
            .filter_map(|name| build_option(name, state.server_speed, main_building_level))
            .collect::<Vec<BuildingOption>>()
    } else {
        vec![]
    };

    let nav_active = if slot_id <= 18 {
        "resources"
    } else {
        "village"
    };

    let template = BuildingTemplate {
        current_user: Some(user),
        nav_active,
        slot_id,
        slot_building,
        available_buildings,
        upgrade,
        current_upkeep,
    };

    render_template(template, None).into_response()
}

fn build_option(
    name: BuildingName,
    server_speed: i8,
    main_building_level: u8,
) -> Option<BuildingOption> {
    let building = Building::new(name.clone(), server_speed);
    let cost = building.cost();
    let time_secs = building.calculate_build_time_secs(&server_speed, &main_building_level);

    Some(BuildingOption {
        name: name.clone(),
        key: format!("{:?}", &name),
        cost: cost.resources.into(),
        upkeep: cost.upkeep,
        time_secs,
        time_formatted: format_duration(time_secs),
    })
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
        time_secs,
        time_formatted: format_duration(time_secs),
    })
}
