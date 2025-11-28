use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;

use parabellum_game::models::{
    buildings::{self, Building},
    village::Village,
};
use parabellum_types::{
    buildings::{BuildingGroup, BuildingName},
    tribe::Tribe,
};

use crate::{
    handlers::{CurrentUser, render_template},
    http::AppState,
    templates::BuildingTemplate,
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
    // TODO:
    // - check if slot is already occupied or if is reserved (wall, main building, rally point)
    //  - show information about the building (value, current level, costs/times for upgrade if available)
    // - if not occupied:
    //  - show a list of unlocked buildings that can be built in that slot

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
        available_buildings: vec![],
    };

    render_template(template, None).into_response()
}
