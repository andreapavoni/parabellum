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
use parabellum_game::models::{
    buildings::{Building, get_building_data},
    smithy::smithy_upgrade_cost_for_unit,
    village::Village,
};
use parabellum_types::{
    army::{UnitGroup, UnitName},
    buildings::{BuildingName, BuildingRequirement},
    common::ResourceGroup,
    tribe::Tribe,
};
use std::collections::{HashMap, HashSet};

use crate::{
    components::{PageLayout, wrap_in_html},
    handlers::helpers::{
        CsrfForm, CurrentUser, HasCsrfToken, generate_csrf, village_movements_or_empty,
        village_queues_or_empty,
    },
    http::AppState,
    pages::buildings::{
        AcademyPage, AcademyQueueItem, AcademyResearchOption, BuildingOption, BuildingValueType,
        EmptySlotPage, ExpansionBuildingPage, GenericBuildingPage, RallyPointPage,
        ResourceFieldPage, SmithyPage, SmithyQueueItem, SmithyUpgradeOption, StaticBuildingPage,
        TrainingBuildingPage, TrainingQueueItem, UnitTrainingOption,
    },
    view_helpers::{
        BuildingQueueItemView, building_queue_to_views, training_queue_to_views, unit_display_name,
    },
};

use super::helpers::create_layout_data;

pub const MAX_SLOT_ID: u8 = 40;

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
pub async fn building_page(
    State(state): State<AppState>,
    Path(slot_id): Path<u8>,
    user: CurrentUser,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    // Validate slot_id
    if !(1..=MAX_SLOT_ID).contains(&slot_id) {
        return Redirect::to("/village").into_response();
    }

    let queues = village_queues_or_empty(&state, user.village.id).await;
    let movements = rally_point_movements_for_slot(&state, &user, slot_id).await;
    let village_info = fetch_village_info_for_rally_point(&state, &user.village).await;

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
        village_info,
    )
    .await;

    (jar, response).into_response()
}

/// POST /dioxus/build/{slot_id} - Execute build/upgrade action
pub async fn build(
    State(state): State<AppState>,
    Path(slot_id): Path<u8>,
    user: CurrentUser,
    CsrfForm { jar, form }: CsrfForm<BuildActionForm>,
) -> Response {
    // Validate slot_id
    if !(1..=MAX_SLOT_ID).contains(&slot_id) {
        return Redirect::to("/village").into_response();
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
        Ok(()) => Redirect::to(&format!("/build/{slot_id}")).into_response(),
        Err(err) => render_with_error(&state, jar, user, slot_id, err.to_string()).await,
    }
}

/// Render building page based on slot contents
async fn render_building_page(
    state: &AppState,
    user: &CurrentUser,
    slot_id: u8,
    csrf_token: String,
    flash_error: Option<String>,
    queues: VillageQueues,
    movements: VillageTroopMovements,
    village_info: std::collections::HashMap<u32, parabellum_app::repository::VillageInfo>,
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

    // Calculate upgrade info accounting for queued upgrades
    let main_building_level = user.village.main_building_level();
    let current_level = slot_building.building.level;

    // Count how many upgrades are queued for this slot
    let building_queue_views = building_queue_to_views(&queues.building);
    let queued_upgrades = building_queue_views
        .iter()
        .filter(|q| q.slot_id == slot_id)
        .count() as u8;

    // The next level to upgrade TO should account for queued upgrades
    let effective_level = current_level.saturating_add(queued_upgrades);
    let next_level = effective_level.saturating_add(1);

    // Just use the queue_full status - user can queue multiple upgrades for same building
    let effective_queue_full = queue_full;

    // Try to calculate upgrade cost - if at max level, we'll still show the page but with disabled upgrade
    let upgrade_info = slot_building
        .building
        .clone()
        .at_level(next_level, state.server_speed)
        .ok();

    let (cost, time_secs, next_upkeep) = if let Some(ref upgraded) = upgrade_info {
        let c = upgraded.cost();
        let t = upgraded.calculate_build_time_secs(&state.server_speed, &main_building_level);
        (c.resources, t, c.upkeep)
    } else {
        // At max level - use dummy values (won't be shown since upgrade is disabled)
        let current_cost = slot_building.building.cost();
        (current_cost.resources, 0, current_cost.upkeep)
    };

    // Calculate formatted next value for display in UpgradeBlock (if upgrade available)
    let next_value_display: Option<String> = upgrade_info.as_ref().map(|upgraded| {
        let value = upgraded.value;
        // Format based on building type
        match slot_building.building.name {
            // Training buildings: divide by 10 and show as percentage
            BuildingName::Barracks
            | BuildingName::GreatBarracks
            | BuildingName::Stable
            | BuildingName::GreatStable
            | BuildingName::Workshop
            | BuildingName::GreatWorkshop => {
                format!("{}%", (value as f32 / 10.0) as u32)
            }
            // Main Building: divide by 10 and show as decimal percentage
            BuildingName::MainBuilding => {
                format!("{:.1}%", value as f32 / 10.0)
            }
            // Production bonus buildings: show as percentage
            BuildingName::Sawmill
            | BuildingName::Brickyard
            | BuildingName::IronFoundry
            | BuildingName::GrainMill
            | BuildingName::Bakery => {
                format!("{}%", value)
            }
            // Defense buildings: show as percentage
            BuildingName::CityWall | BuildingName::EarthWall | BuildingName::Palisade => {
                format!("{}%", value)
            }
            // Resource fields and storage: show as integer
            BuildingName::Woodcutter
            | BuildingName::ClayPit
            | BuildingName::IronMine
            | BuildingName::Cropland
            | BuildingName::Warehouse
            | BuildingName::Granary
            | BuildingName::GreatWarehouse
            | BuildingName::GreatGranary
            | BuildingName::Cranny => {
                format!("{}", value)
            }
            // Other buildings: no specific value display needed
            _ => format!("{}", value),
        }
    });

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
                        cost: cost,
                        time_secs: time_secs,
                        next_upkeep: next_upkeep,
                        queue_full: effective_queue_full,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                        next_value: next_value_display.clone(),
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
                        current_value: slot_building.building.value,
                        population: slot_building.building.population,
                        next_level: next_level,
                        cost: cost,
                        time_secs: time_secs,
                        current_upkeep: slot_building.building.cost().upkeep,
                        next_upkeep: next_upkeep,
                        queue_full: effective_queue_full,
                        training_units: training_units,
                        training_queue: training_queue,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                        next_value: next_value_display.clone(),
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
                        current_value: slot_building.building.value,
                        population: slot_building.building.population,
                        next_level: next_level,
                        cost: cost,
                        time_secs: time_secs,
                        current_upkeep: slot_building.building.cost().upkeep,
                        next_upkeep: next_upkeep,
                        queue_full: effective_queue_full,
                        training_units: training_units,
                        training_queue: training_queue,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                        next_value: next_value_display.clone(),
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
                        current_value: slot_building.building.value,
                        population: slot_building.building.population,
                        next_level: next_level,
                        cost: cost,
                        time_secs: time_secs,
                        current_upkeep: slot_building.building.cost().upkeep,
                        next_upkeep: next_upkeep,
                        queue_full: effective_queue_full,
                        training_units: training_units,
                        training_queue: training_queue,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                        next_value: next_value_display.clone(),
                    }
                }
            })
        }
        BuildingName::Academy => {
            // Research units
            let (ready_units, locked_units, researched_units) =
                academy_options_for_village(&user.village, state.server_speed, &queues.academy);
            let academy_queue = academy_queue_for_slot(&queues.academy);
            let academy_queue_full = queues.academy.len() >= 2;

            dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data,
                    AcademyPage {
                        village: user.village.clone(),
                        slot_id: slot_id,
                        building_name: slot_building.building.name.clone(),
                        current_level: slot_building.building.level,
                        population: slot_building.building.population,
                        next_level: next_level,
                        cost: cost,
                        time_secs: time_secs,
                        current_upkeep: slot_building.building.cost().upkeep,
                        next_upkeep: next_upkeep,
                        queue_full: effective_queue_full,
                        ready_units: ready_units,
                        locked_units: locked_units,
                        researched_units: researched_units,
                        academy_queue: academy_queue,
                        academy_queue_full: academy_queue_full,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                        next_value: next_value_display.clone(),
                    }
                }
            })
        }
        BuildingName::Smithy => {
            // Upgrade units
            let smithy_queue_full = queues.smithy.len() >= 2;
            let smithy_units = smithy_options_for_village(
                &user.village,
                &slot_building,
                state.server_speed,
                &queues.smithy,
                smithy_queue_full,
            );
            let smithy_queue = smithy_queue_for_slot(&queues.smithy);

            dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data,
                    SmithyPage {
                        village: user.village.clone(),
                        slot_id: slot_id,
                        building_name: slot_building.building.name.clone(),
                        current_level: slot_building.building.level,
                        population: slot_building.building.population,
                        next_level: next_level,
                        cost: cost,
                        time_secs: time_secs,
                        current_upkeep: slot_building.building.cost().upkeep,
                        next_upkeep: next_upkeep,
                        queue_full: effective_queue_full,
                        smithy_units: smithy_units,
                        smithy_queue: smithy_queue,
                        smithy_queue_full: smithy_queue_full,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                        next_value: next_value_display.clone(),
                    }
                }
            })
        }
        BuildingName::RallyPoint => {
            // Rally point - troop movements and sending
            dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data,
                    RallyPointPage {
                        village: user.village.clone(),
                        slot_id: slot_id,
                        building_name: slot_building.building.name.clone(),
                        current_level: slot_building.building.level,
                        population: slot_building.building.population,
                        next_level: next_level,
                        cost: cost,
                        time_secs: time_secs,
                        current_upkeep: slot_building.building.cost().upkeep,
                        next_upkeep: next_upkeep,
                        queue_full: effective_queue_full,
                        movements: movements,
                        village_info: village_info,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                        next_value: next_value_display.clone(),
                    }
                }
            })
        }
        // Static buildings with value display
        BuildingName::Warehouse | BuildingName::GreatWarehouse => render_static_building(
            layout_data,
            user,
            slot_id,
            &slot_building,
            next_level,
            cost,
            time_secs,
            next_upkeep,
            effective_queue_full,
            &upgrade_info,
            BuildingValueType::Capacity,
            csrf_token,
            flash_error,
        ),
        BuildingName::Granary | BuildingName::GreatGranary => render_static_building(
            layout_data,
            user,
            slot_id,
            &slot_building,
            next_level,
            cost,
            time_secs,
            next_upkeep,
            effective_queue_full,
            &upgrade_info,
            BuildingValueType::Capacity,
            csrf_token,
            flash_error,
        ),
        BuildingName::Sawmill => render_static_building(
            layout_data,
            user,
            slot_id,
            &slot_building,
            next_level,
            cost,
            time_secs,
            next_upkeep,
            effective_queue_full,
            &upgrade_info,
            BuildingValueType::ProductionBonus {
                resource_type: "Lumber",
            },
            csrf_token,
            flash_error,
        ),
        BuildingName::Brickyard => render_static_building(
            layout_data,
            user,
            slot_id,
            &slot_building,
            next_level,
            cost,
            time_secs,
            next_upkeep,
            effective_queue_full,
            &upgrade_info,
            BuildingValueType::ProductionBonus {
                resource_type: "Clay",
            },
            csrf_token,
            flash_error,
        ),
        BuildingName::IronFoundry => render_static_building(
            layout_data,
            user,
            slot_id,
            &slot_building,
            next_level,
            cost,
            time_secs,
            next_upkeep,
            effective_queue_full,
            &upgrade_info,
            BuildingValueType::ProductionBonus {
                resource_type: "Iron",
            },
            csrf_token,
            flash_error,
        ),
        BuildingName::GrainMill | BuildingName::Bakery => render_static_building(
            layout_data,
            user,
            slot_id,
            &slot_building,
            next_level,
            cost,
            time_secs,
            next_upkeep,
            effective_queue_full,
            &upgrade_info,
            BuildingValueType::ProductionBonus {
                resource_type: "Crop",
            },
            csrf_token,
            flash_error,
        ),
        BuildingName::MainBuilding => render_static_building(
            layout_data,
            user,
            slot_id,
            &slot_building,
            next_level,
            cost,
            time_secs,
            next_upkeep,
            effective_queue_full,
            &upgrade_info,
            BuildingValueType::ConstructionSpeedBonus,
            csrf_token,
            flash_error,
        ),
        BuildingName::Cranny => render_static_building(
            layout_data,
            user,
            slot_id,
            &slot_building,
            next_level,
            cost,
            time_secs,
            next_upkeep,
            effective_queue_full,
            &upgrade_info,
            BuildingValueType::HiddenCapacity,
            csrf_token,
            flash_error,
        ),
        BuildingName::CityWall | BuildingName::EarthWall | BuildingName::Palisade => {
            render_static_building(
                layout_data,
                user,
                slot_id,
                &slot_building,
                next_level,
                cost,
                time_secs,
                next_upkeep,
                effective_queue_full,
                &upgrade_info,
                BuildingValueType::DefenseBonus,
                csrf_token,
                flash_error,
            )
        }
        BuildingName::Residence | BuildingName::Palace => {
            // Expansion buildings - show culture points info
            use parabellum_app::cqrs::queries::GetCulturePointsInfo;
            use parabellum_app::queries_handlers::GetCulturePointsInfoQueryHandler;

            let village_cpp = user.village.culture_points_production;
            let (account_cpp, account_cp) = state
                .app_bus
                .query(
                    GetCulturePointsInfo {
                        player_id: user.player.id,
                    },
                    GetCulturePointsInfoQueryHandler::new(),
                )
                .await
                .map(|info| {
                    (
                        info.account_culture_points_production,
                        info.account_culture_points,
                    )
                })
                .unwrap_or((0, 0));

            dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data,
                    ExpansionBuildingPage {
                        village: user.village.clone(),
                        slot_id: slot_id,
                        building_name: slot_building.building.name.clone(),
                        current_level: current_level,
                        next_level: next_level,
                        cost: cost,
                        time_secs: time_secs,
                        current_upkeep: slot_building.building.cost().upkeep,
                        next_upkeep: next_upkeep,
                        queue_full: effective_queue_full,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                        village_culture_points_production: village_cpp,
                        account_culture_points_production: account_cpp,
                        account_culture_points: account_cp,
                        next_value: next_value_display.clone(),
                    }
                }
            })
        }
        _ => {
            // Generic buildings - just upgrade block
            dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data,
                    GenericBuildingPage {
                        village: user.village.clone(),
                        slot_id: slot_id,
                        building_name: slot_building.building.name.clone(),
                        current_level: current_level,
                        next_level: next_level,
                        cost: cost,
                        time_secs: time_secs,
                        current_upkeep: slot_building.building.cost().upkeep,
                        next_upkeep: next_upkeep,
                        queue_full: effective_queue_full,
                        csrf_token: csrf_token,
                        flash_error: flash_error,
                        next_value: next_value_display.clone(),
                    }
                }
            })
        }
    };

    Html(wrap_in_html(&body_content)).into_response()
}

/// Helper to render a static building page (buildings with meaningful value display)
fn render_static_building(
    layout_data: crate::components::LayoutData,
    user: &CurrentUser,
    slot_id: u8,
    slot_building: &parabellum_game::models::village::VillageBuilding,
    next_level: u8,
    cost: ResourceGroup,
    time_secs: u32,
    next_upkeep: u32,
    effective_queue_full: bool,
    upgrade_info: &Option<parabellum_game::models::buildings::Building>,
    value_type: BuildingValueType,
    csrf_token: String,
    flash_error: Option<String>,
) -> String {
    let next_value = upgrade_info.as_ref().map(|b| b.value);

    dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            StaticBuildingPage {
                village: user.village.clone(),
                slot_id: slot_id,
                building_name: slot_building.building.name.clone(),
                current_level: slot_building.building.level,
                current_value: slot_building.building.value,
                next_value: next_value,
                value_type: value_type,
                population: slot_building.building.population,
                next_level: next_level,
                cost: cost,
                time_secs: time_secs,
                current_upkeep: slot_building.building.cost().upkeep,
                next_upkeep: next_upkeep,
                queue_full: effective_queue_full,
                csrf_token: csrf_token,
                flash_error: flash_error,
            }
        }
    })
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

/// Collect village IDs referenced by armies and fetch their info
async fn fetch_village_info_for_rally_point(
    state: &AppState,
    village: &Village,
) -> std::collections::HashMap<u32, parabellum_app::repository::VillageInfo> {
    use std::collections::HashSet;

    let mut village_ids = HashSet::new();

    // Collect IDs from deployed armies
    for army in village.deployed_armies() {
        if let Some(dest_id) = army.current_map_field_id {
            village_ids.insert(dest_id);
        }
    }

    // Collect IDs from reinforcements
    for reinforcement in village.reinforcements() {
        village_ids.insert(reinforcement.village_id);
    }

    // Fetch village info if we have IDs to look up
    if village_ids.is_empty() {
        return std::collections::HashMap::new();
    }

    let ids: Vec<u32> = village_ids.into_iter().collect();

    use parabellum_app::cqrs::queries::GetVillageInfoByIds;
    use parabellum_app::queries_handlers::GetVillageInfoByIdsHandler;

    state
        .app_bus
        .query(
            GetVillageInfoByIds { village_ids: ids },
            GetVillageInfoByIdsHandler::new(),
        )
        .await
        .unwrap_or_default()
}

/// Render page with error message
pub async fn render_with_error(
    state: &AppState,
    jar: SignedCookieJar,
    user: CurrentUser,
    slot_id: u8,
    error: String,
) -> Response {
    let queues = village_queues_or_empty(state, user.village.id).await;
    let movements = rally_point_movements_for_slot(state, &user, slot_id).await;
    let village_info = fetch_village_info_for_rally_point(state, &user.village).await;
    let (_jar, csrf_token) = generate_csrf(jar);

    render_building_page(
        state,
        &user,
        slot_id,
        csrf_token,
        Some(error),
        queues,
        movements,
        village_info,
    )
    .await
}

/// Calculate building options for an empty slot
fn building_options_for_slot(
    village: &parabellum_game::models::village::Village,
    slot_id: u8,
    queue_views: &[BuildingQueueItemView],
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
            cost: cost.resources.clone(),
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

fn building_blocked_by_queue(name: &BuildingName, queue: &[BuildingQueueItemView]) -> bool {
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

/// Calculate academy research options for village
fn academy_options_for_village(
    village: &parabellum_game::models::village::Village,
    server_speed: i8,
    queued_jobs: &[parabellum_app::cqrs::queries::AcademyQueueItem],
) -> (
    Vec<AcademyResearchOption>,
    Vec<AcademyResearchOption>,
    Vec<String>,
) {
    let mut ready = Vec::new();
    let mut locked = Vec::new();
    let mut researched = Vec::new();
    let research = village.academy_research();
    let units = village.tribe.units();
    let queued_units: HashSet<UnitName> = queued_jobs.iter().map(|job| job.unit.clone()).collect();

    for (idx, unit) in units.iter().enumerate() {
        let is_researched = research.get(idx);
        let is_queued = queued_units.contains(&unit.name);
        let missing_requirements = missing_unit_requirements(village, unit.get_requirements());
        let can_research = !is_researched && missing_requirements.is_empty();
        let time_secs = if unit.research_cost.time == 0 {
            0
        } else {
            ((unit.research_cost.time as f64 / server_speed as f64)
                .floor()
                .max(1.0)) as u32
        };

        if is_researched {
            researched.push(unit_display_name(&unit.name));
        } else {
            let option = AcademyResearchOption {
                unit_name: unit_display_name(&unit.name),
                unit_value: format!("{:?}", unit.name),
                cost: unit.research_cost.resources.clone(),
                time_secs,
                missing_requirements,
            };

            if can_research && !is_queued {
                ready.push(option);
            } else if !can_research {
                locked.push(option);
            }
        }
    }

    (ready, locked, researched)
}

/// Get academy queue items
fn academy_queue_for_slot(
    queue: &[parabellum_app::cqrs::queries::AcademyQueueItem],
) -> Vec<AcademyQueueItem> {
    use parabellum_app::jobs::JobStatus;

    queue
        .iter()
        .map(|item| {
            let now = chrono::Utc::now();
            let time_remaining_secs = (item.finishes_at - now).num_seconds().max(0) as u32;

            AcademyQueueItem {
                unit_name: unit_display_name(&item.unit),
                time_remaining_secs,
                is_processing: item.status == JobStatus::Processing,
            }
        })
        .collect()
}

/// Helper to convert building requirements to tuple format
fn missing_unit_requirements(
    village: &parabellum_game::models::village::Village,
    requirements: &[BuildingRequirement],
) -> Vec<(BuildingName, u8)> {
    requirements
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

/// Calculate smithy upgrade options for village
fn smithy_options_for_village(
    village: &parabellum_game::models::village::Village,
    smithy_building: &parabellum_game::models::village::VillageBuilding,
    server_speed: i8,
    queue: &[parabellum_app::cqrs::queries::SmithyQueueItem],
    queue_full: bool,
) -> Vec<SmithyUpgradeOption> {
    let smithy_level_cap = smithy_building.building.level.min(20);
    if smithy_level_cap == 0 {
        return vec![];
    }

    let queue_counts = smithy_queue_counts(queue);
    let research = village.academy_research();
    let smithy_levels = village.smithy();
    let units = village.tribe.units();
    let mut options = Vec::new();

    for (idx, unit) in units.iter().enumerate() {
        if idx >= smithy_levels.len() {
            break;
        }

        let is_researched = unit.research_cost.time == 0 || research.get(idx);
        let current_level = smithy_levels[idx];
        let queued = queue_counts.get(&unit.name).copied().unwrap_or(0);
        let effective_level = current_level.saturating_add(queued);
        let available_for_upgrade =
            is_researched && effective_level < smithy_level_cap && smithy_level_cap > 0;
        let can_upgrade = available_for_upgrade && queued == 0 && !queue_full;

        if !is_researched {
            continue;
        }

        let (cost, time_secs) = if available_for_upgrade {
            match smithy_upgrade_cost_for_unit(&unit.name, effective_level) {
                Ok(research_cost) => {
                    let time = if research_cost.time == 0 {
                        0
                    } else {
                        ((research_cost.time as f64 / server_speed as f64)
                            .floor()
                            .max(1.0)) as u32
                    };
                    (research_cost.resources, time)
                }
                Err(_) => continue,
            }
        } else {
            continue;
        };

        options.push(SmithyUpgradeOption {
            unit_name: unit_display_name(&unit.name),
            unit_value: format!("{:?}", unit.name),
            current_level,
            max_level: smithy_level_cap,
            cost,
            time_secs,
            can_upgrade,
        });
    }

    options
}

/// Count queued smithy upgrades per unit
fn smithy_queue_counts(
    queue: &[parabellum_app::cqrs::queries::SmithyQueueItem],
) -> HashMap<UnitName, u8> {
    let mut counts = HashMap::new();
    for job in queue {
        *counts.entry(job.unit.clone()).or_insert(0) += 1;
    }
    counts
}

/// Get smithy queue items
fn smithy_queue_for_slot(
    queue: &[parabellum_app::cqrs::queries::SmithyQueueItem],
) -> Vec<SmithyQueueItem> {
    use parabellum_app::jobs::JobStatus;

    // Count how many times each unit appears in queue to calculate target level
    let mut unit_counts: HashMap<UnitName, u8> = HashMap::new();

    queue
        .iter()
        .map(|item| {
            let now = chrono::Utc::now();
            let time_remaining_secs = (item.finishes_at - now).num_seconds().max(0) as u32;

            // Increment count for this unit to determine target level
            let count = unit_counts.entry(item.unit.clone()).or_insert(0);
            *count += 1;
            let target_level = *count;

            SmithyQueueItem {
                unit_name: unit_display_name(&item.unit),
                target_level,
                time_remaining_secs,
                is_processing: item.status == JobStatus::Processing,
            }
        })
        .collect()
}
