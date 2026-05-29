//! Building detail read endpoint and related DTOs.
//!
//! This module serves `GET /api/v1/buildings/{slot_id}` with a rich, canonical payload
//! describing the target slot plus endpoint-specific sections (training, expansion,
//! academy, smithy, marketplace, rally point).

use std::collections::{HashMap, HashSet};

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
};
use chrono::Utc;
use serde::Serialize;

use parabellum_app::{
    ports::queries::{
        AcademyQueueItem, BuildingQueueItem, SmithyQueueItem, TrainingQueueItem,
        VillageArmyStateView,
    },
    read_models::VillageInfo,
    villages::models::ScheduledActionStatus,
};
use parabellum_game::models::{
    buildings::{Building, get_building_data},
    smithy::smithy_upgrade_cost_for_unit,
    village::VillageBuilding,
};
use parabellum_types::{
    army::{UnitGroup, UnitName, UnitRole},
    buildings::{BuildingName, BuildingRequirement},
    common::ResourceGroup,
    errors::ApplicationError,
    tribe::Tribe,
};

use crate::{
    api::{
        dto::{ResourceAmountsDto, village_list, village_summary},
        errors::ApiError,
    },
    http::AppState,
    view_helpers::{
        MerchantMovementDirection, building_description_paragraphs, prepare_global_offers,
        prepare_merchant_movements, prepare_own_offers, prepare_rally_point_cards,
    },
};

use super::authenticated_user;
use super::error_mapping::map_application_error;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Response payload for `GET /api/v1/buildings/{slot_id}`.
pub struct BuildingPageResponse {
    pub server_time: i64,
    pub village: crate::api::dto::VillageSummaryDto,
    pub villages: Vec<crate::api::dto::VillageListItemDto>,
    pub detail: BuildingDetailDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Shared building detail envelope; optional sections depend on building type.
pub struct BuildingDetailDto {
    pub slot_id: u8,
    pub village_id: u32,
    pub building_name: String,
    pub building_type: BuildingTypeDto,
    pub current_level: u8,
    pub population: u32,
    pub current_upkeep: u32,
    pub next_level: u8,
    pub next_upkeep: u32,
    pub time_secs: u32,
    pub queue_full: bool,
    pub at_max_level: bool,
    pub next_value: Option<String>,
    pub cost: ResourceAmountsDto,
    pub stored_resources: ResourceAmountsDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_slot: Option<EmptySlotDetailDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training: Option<TrainingDetailDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expansion: Option<ExpansionDetailDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub academy: Option<AcademyDetailDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smithy: Option<SmithyDetailDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marketplace: Option<MarketplaceDetailDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rally_point: Option<RallyPointDetailDto>,
    pub description_paragraphs: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
/// Runtime category for the selected slot/building.
pub enum BuildingTypeDto {
    Empty,
    Generic,
    Training,
    Expansion,
    Academy,
    Smithy,
    Marketplace,
    RallyPoint,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpansionDetailDto {
    pub village_culture_points_production: u32,
    pub account_culture_points_production: u32,
    pub account_culture_points: u32,
    pub next_cp_required: u32,
    pub max_foundation_slots: u8,
    pub child_villages_count: u32,
    pub settlers_at_home: u32,
    pub settlers_deployed: u32,
    pub max_settlers_trainable: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptySlotDetailDto {
    pub buildable_buildings: Vec<BuildOptionDto>,
    pub locked_buildings: Vec<BuildOptionDto>,
    pub has_queue_for_slot: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queued_building_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queued_target_level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queued_next_level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queued_can_upgrade: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queued_upgrade_preview: Option<QueuedUpgradePreviewDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedUpgradePreviewDto {
    pub building_name: String,
    pub current_level: u8,
    pub next_level: u8,
    pub current_upkeep: u32,
    pub next_upkeep: u32,
    pub time_secs: u32,
    pub at_max_level: bool,
    pub next_value: Option<String>,
    pub cost: ResourceAmountsDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildOptionDto {
    pub building_name: String,
    pub cost: ResourceAmountsDto,
    pub time_secs: u32,
    pub missing_requirements: Vec<RequirementDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingDetailDto {
    pub training_speed_percent: u32,
    pub units: Vec<TrainingUnitOptionDto>,
    pub queue: Vec<TrainingQueueItemDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingUnitOptionDto {
    pub unit_idx: u8,
    pub name: String,
    pub cost: ResourceAmountsDto,
    pub upkeep: u32,
    pub attack: u32,
    pub defense_infantry: u32,
    pub defense_cavalry: u32,
    pub speed: u8,
    pub capacity: u32,
    pub time_secs: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingQueueItemDto {
    pub quantity: u32,
    pub unit_name: String,
    pub time_per_unit: u32,
    pub finishes_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcademyDetailDto {
    pub ready_units: Vec<AcademyResearchOptionDto>,
    pub locked_units: Vec<AcademyResearchOptionDto>,
    pub researched_units: Vec<String>,
    pub queue: Vec<AcademyQueueItemDto>,
    pub queue_full: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcademyResearchOptionDto {
    pub unit_name: String,
    pub cost: ResourceAmountsDto,
    pub time_secs: u32,
    pub missing_requirements: Vec<RequirementDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcademyQueueItemDto {
    pub unit_name: String,
    pub finishes_at: chrono::DateTime<chrono::Utc>,
    pub is_processing: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmithyDetailDto {
    pub units: Vec<SmithyUpgradeOptionDto>,
    pub queue: Vec<SmithyQueueItemDto>,
    pub queue_full: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmithyUpgradeOptionDto {
    pub unit_name: String,
    pub current_level: u8,
    pub max_level: u8,
    pub cost: ResourceAmountsDto,
    pub time_secs: u32,
    pub can_upgrade: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmithyQueueItemDto {
    pub unit_name: String,
    pub target_level: u8,
    pub finishes_at: chrono::DateTime<chrono::Utc>,
    pub is_processing: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementDto {
    pub building_name: String,
    pub required_level: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Marketplace section payload in building detail response.
pub struct MarketplaceDetailDto {
    pub available_merchants: u8,
    pub total_merchants: u8,
    pub own_offers: Vec<MarketplaceOfferDto>,
    pub global_offers: Vec<MarketplaceOfferDto>,
    pub merchant_movements: Vec<MerchantMovementDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceOfferDto {
    pub offer_id: String,
    pub village_id: u32,
    pub village_name: String,
    pub position: PositionDto,
    pub offer_resources: ResourceAmountsDto,
    pub seek_resources: ResourceAmountsDto,
    pub merchants_required: u8,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MerchantMovementDirectionDto {
    Outgoing,
    Incoming,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MerchantMovementKindDto {
    Going,
    Return,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantMovementDto {
    pub job_id: String,
    pub direction: MerchantMovementDirectionDto,
    pub kind: MerchantMovementKindDto,
    pub origin_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_position: Option<PositionDto>,
    pub destination_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_position: Option<PositionDto>,
    pub resources: ResourceAmountsDto,
    pub merchants_used: u8,
    pub arrives_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Rally point section payload in building detail response.
pub struct RallyPointDetailDto {
    pub cards: Vec<RallyCardDto>,
    pub sendable_units: Vec<RallySendableUnitDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RallyCardCategoryDto {
    Stationed,
    Reinforcement,
    Deployed,
    Incoming,
    Outgoing,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RallyMovementKindDto {
    Attack,
    Raid,
    Scout,
    Reinforcement,
    Return,
    FoundVillage,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RallyActionDto {
    Recall,
    Release,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RallyCardDto {
    pub village_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub village_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<PositionDto>,
    pub tribe: String,
    pub units: Vec<u32>,
    pub upkeep: u32,
    pub category: RallyCardCategoryDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub movement_kind: Option<RallyMovementKindDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrives_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounty: Option<ResourceAmountsDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<RallyActionDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RallySendableUnitDto {
    pub unit_idx: usize,
    pub name: String,
    pub available: u32,
    pub is_researched: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionDto {
    pub x: i32,
    pub y: i32,
}

/// Returns building detail payload for one slot in current authenticated village.
#[utoipa::path(
    get,
    path = "/buildings/{slot_id}",
    params(
        ("slot_id" = u8, Path, description = "Building slot id (1..=40)")
    ),
    responses(
        (status = 200, body = serde_json::Value, description = "Building page response")
    )
)]
pub async fn building_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slot_id): Path<u8>,
) -> Result<impl IntoResponse, ApiError> {
    if !(1..=40).contains(&slot_id) {
        return Err(ApiError::not_found("Building slot not found"));
    }

    let user = authenticated_user(&state, &headers).await?;
    let queues = state
        .game_app
        .get_village_queues(user.village.id)
        .await
        .map_err(|err| map_application_error("unable_to_load_village_queues", err))?;
    let stored = user.village.stored_resources();
    let building_queue_capacity: usize =
        if matches!(user.village.tribe, parabellum_types::tribe::Tribe::Roman) {
            3
        } else {
            2
        };
    let queue_full = queues.building.len() >= building_queue_capacity;
    let slot = user.village.get_building_by_slot_id(slot_id);

    let detail = if let Some(slot) = slot {
        let building_type = match slot.building.name {
            BuildingName::Barracks
            | BuildingName::GreatBarracks
            | BuildingName::Stable
            | BuildingName::GreatStable
            | BuildingName::Workshop
            | BuildingName::GreatWorkshop => BuildingTypeDto::Training,
            BuildingName::Residence | BuildingName::Palace => BuildingTypeDto::Expansion,
            BuildingName::Academy => BuildingTypeDto::Academy,
            BuildingName::Smithy => BuildingTypeDto::Smithy,
            BuildingName::Marketplace => BuildingTypeDto::Marketplace,
            BuildingName::RallyPoint => BuildingTypeDto::RallyPoint,
            _ => BuildingTypeDto::Generic,
        };

        let main_building_level = user.village.main_building_level();
        let current_level = slot.building.level;
        let queued_upgrades = queues
            .building
            .iter()
            .filter(|item| item.slot_id == slot_id)
            .count() as u8;
        let max_level = get_building_data(&slot.building.name)
            .map(|data| data.rules.max_level)
            .unwrap_or(current_level);
        let pending_level = current_level.saturating_add(queued_upgrades);
        let at_max_level = pending_level >= max_level;
        let next_level = pending_level.saturating_add(1).min(max_level);

        let upgrade_info = if at_max_level {
            None
        } else {
            slot.building
                .clone()
                .at_level(next_level, state.server_speed)
                .ok()
        };
        let (cost, time_secs, next_upkeep) = if let Some(ref upgraded) = upgrade_info {
            let computed = upgraded.cost();
            (
                computed.resources,
                upgraded.calculate_build_time_secs(&state.server_speed, &main_building_level),
                computed.upkeep,
            )
        } else {
            let current_cost = slot.building.cost();
            (current_cost.resources, 0, current_cost.upkeep)
        };

        let next_value = upgrade_info
            .as_ref()
            .map(|upgraded| format_next_value(slot.building.name.clone(), upgraded.value));
        let (chiefs_moving, settlers_moving) = state
            .game_app
            .get_village_troop_movements(user.village.id)
            .await
            .map(|movements| {
                movements
                    .outgoing
                    .iter()
                    .chain(movements.incoming.iter())
                    .filter(|m| m.origin_player_id == user.player.id)
                    .fold((0u32, 0u32), |(chiefs, settlers), m| {
                        (
                            chiefs.saturating_add(m.units.get(8)),
                            settlers.saturating_add(m.units.get(9)),
                        )
                    })
            })
            .unwrap_or((0, 0));

        match building_type {
            BuildingTypeDto::Empty => unreachable!(),
            BuildingTypeDto::Generic => BuildingDetailDto {
                slot_id,
                village_id: user.village.id,
                building_name: building_key(&slot.building.name),
                building_type: BuildingTypeDto::Generic,
                current_level,
                population: slot.building.population,
                current_upkeep: slot.building.cost().upkeep,
                next_level,
                next_upkeep,
                time_secs,
                queue_full,
                at_max_level,
                next_value,
                cost: resource_group_to_dto(&cost),
                stored_resources: resource_group_to_dto(&stored),
                empty_slot: None,
                training: None,
                expansion: None,
                academy: None,
                smithy: None,
                marketplace: None,
                rally_point: None,
                description_paragraphs: building_description_paragraphs(&slot.building.name),
            },
            BuildingTypeDto::Training => {
                let (expected_buildings, group) = match slot.building.name {
                    BuildingName::Barracks | BuildingName::GreatBarracks => (
                        vec![BuildingName::Barracks, BuildingName::GreatBarracks],
                        UnitGroup::Infantry,
                    ),
                    BuildingName::Stable | BuildingName::GreatStable => (
                        vec![BuildingName::Stable, BuildingName::GreatStable],
                        UnitGroup::Cavalry,
                    ),
                    BuildingName::Workshop | BuildingName::GreatWorkshop => (
                        vec![BuildingName::Workshop, BuildingName::GreatWorkshop],
                        UnitGroup::Siege,
                    ),
                    _ => unreachable!(),
                };

                let units = training_options_for_group(
                    &user.village,
                    state.server_speed,
                    &slot,
                    &expected_buildings,
                    group,
                    &queues.training,
                    None,
                    (chiefs_moving, settlers_moving),
                );
                let queue = training_queue_for_slot(slot_id, &queues.training);

                BuildingDetailDto {
                    slot_id,
                    village_id: user.village.id,
                    building_name: building_key(&slot.building.name),
                    building_type: BuildingTypeDto::Training,
                    current_level,
                    population: slot.building.population,
                    current_upkeep: slot.building.cost().upkeep,
                    next_level,
                    next_upkeep,
                    time_secs,
                    queue_full,
                    at_max_level,
                    next_value,
                    cost: resource_group_to_dto(&cost),
                    stored_resources: resource_group_to_dto(&stored),
                    empty_slot: None,
                    training: Some(TrainingDetailDto {
                        training_speed_percent: (slot.building.value as f32 / 10.0) as u32,
                        units,
                        queue,
                    }),
                    expansion: None,
                    academy: None,
                    smithy: None,
                    marketplace: None,
                    rally_point: None,
                    description_paragraphs: building_description_paragraphs(&slot.building.name),
                }
            }
            BuildingTypeDto::Expansion => {
                let culture_points_info = state
                    .game_app
                    .get_expansion_culture_info(user.player.id, user.village.id, state.server_speed)
                    .await
                    .map_err(|err| map_application_error("unable_to_load_expansion_info", err))?;
                let account_culture_points_production =
                    culture_points_info.player_culture_points_production;
                let account_culture_points = culture_points_info.player_culture_points;
                let village_culture_points_production =
                    culture_points_info.village_culture_points_production;
                let next_cp_required = culture_points_info.next_cp_required;

                let max_slots = user.village.max_foundation_slots();
                let child_villages_count = if max_slots > 0 {
                    user.villages
                        .iter()
                        .filter(|v| v.parent_village_id == Some(user.village.id))
                        .count() as u32
                } else {
                    0
                };
                let available_slots = max_slots.saturating_sub(child_villages_count as u8);
                let training_units = training_options_for_group(
                    &user.village,
                    state.server_speed,
                    &slot,
                    &[BuildingName::Residence, BuildingName::Palace],
                    UnitGroup::Expansion,
                    &queues.training,
                    Some(available_slots),
                    (chiefs_moving, settlers_moving),
                );
                let training_queue = training_queue_for_slot(slot_id, &queues.training);
                let settler_idx = user
                    .village
                    .tribe
                    .get_unit_idx_by_name(&UnitName::Settler)
                    .unwrap_or(9);
                let army_state = state
                    .game_app
                    .get_village_army_state_view(user.village.id)
                    .await
                    .map_err(|err| {
                        map_application_error("unable_to_load_village_army_state", err)
                    })?;
                let settlers_at_home = army_state
                    .home_army
                    .as_ref()
                    .map(|a| a.units().get(settler_idx))
                    .unwrap_or(0);
                let settlers_deployed: u32 = army_state
                    .deployed_armies
                    .iter()
                    .map(|army| army.units().get(settler_idx))
                    .sum();
                let max_settlers_trainable = if available_slots > 0 {
                    (available_slots as u32 * 3)
                        .saturating_sub(settlers_at_home + settlers_deployed)
                } else {
                    0
                };

                BuildingDetailDto {
                    slot_id,
                    village_id: user.village.id,
                    building_name: building_key(&slot.building.name),
                    building_type: BuildingTypeDto::Expansion,
                    current_level,
                    population: slot.building.population,
                    current_upkeep: slot.building.cost().upkeep,
                    next_level,
                    next_upkeep,
                    time_secs,
                    queue_full,
                    at_max_level,
                    next_value,
                    cost: resource_group_to_dto(&cost),
                    stored_resources: resource_group_to_dto(&stored),
                    empty_slot: None,
                    training: Some(TrainingDetailDto {
                        training_speed_percent: (slot.building.value as f32 / 10.0) as u32,
                        units: training_units,
                        queue: training_queue,
                    }),
                    expansion: Some(ExpansionDetailDto {
                        village_culture_points_production,
                        account_culture_points_production,
                        account_culture_points,
                        next_cp_required,
                        max_foundation_slots: max_slots,
                        child_villages_count,
                        settlers_at_home,
                        settlers_deployed,
                        max_settlers_trainable,
                    }),
                    academy: None,
                    smithy: None,
                    marketplace: None,
                    rally_point: None,
                    description_paragraphs: building_description_paragraphs(&slot.building.name),
                }
            }
            BuildingTypeDto::Academy => {
                let (ready_units, locked_units, researched_units) =
                    academy_options_for_village(&user.village, state.server_speed, &queues.academy);
                let queue = academy_queue_for_slot(&queues.academy);
                let queue_full_academy = queues.academy.len() >= 2;

                BuildingDetailDto {
                    slot_id,
                    village_id: user.village.id,
                    building_name: building_key(&slot.building.name),
                    building_type: BuildingTypeDto::Academy,
                    current_level,
                    population: slot.building.population,
                    current_upkeep: slot.building.cost().upkeep,
                    next_level,
                    next_upkeep,
                    time_secs,
                    queue_full,
                    at_max_level,
                    next_value,
                    cost: resource_group_to_dto(&cost),
                    stored_resources: resource_group_to_dto(&stored),
                    empty_slot: None,
                    training: None,
                    expansion: None,
                    academy: Some(AcademyDetailDto {
                        ready_units,
                        locked_units,
                        researched_units,
                        queue,
                        queue_full: queue_full_academy,
                    }),
                    smithy: None,
                    marketplace: None,
                    rally_point: None,
                    description_paragraphs: building_description_paragraphs(&slot.building.name),
                }
            }
            BuildingTypeDto::Smithy => {
                let queue_full_smithy = queues.smithy.len() >= 2;
                let units = smithy_options_for_village(
                    &user.village,
                    &slot,
                    state.server_speed,
                    &queues.smithy,
                    queue_full_smithy,
                );
                let queue = smithy_queue_for_slot(&queues.smithy);

                BuildingDetailDto {
                    slot_id,
                    village_id: user.village.id,
                    building_name: building_key(&slot.building.name),
                    building_type: BuildingTypeDto::Smithy,
                    current_level,
                    population: slot.building.population,
                    current_upkeep: slot.building.cost().upkeep,
                    next_level,
                    next_upkeep,
                    time_secs,
                    queue_full,
                    at_max_level,
                    next_value,
                    cost: resource_group_to_dto(&cost),
                    stored_resources: resource_group_to_dto(&stored),
                    empty_slot: None,
                    training: None,
                    expansion: None,
                    academy: None,
                    smithy: Some(SmithyDetailDto {
                        units,
                        queue,
                        queue_full: queue_full_smithy,
                    }),
                    marketplace: None,
                    rally_point: None,
                    description_paragraphs: building_description_paragraphs(&slot.building.name),
                }
            }
            BuildingTypeDto::Marketplace => {
                let marketplace_data = state
                    .game_app
                    .get_marketplace_data(user.village.id)
                    .await
                    .map_err(|err| {
                    map_application_error("unable_to_load_marketplace_data", err)
                })?;

                let own_offers = prepare_own_offers(&marketplace_data)
                    .into_iter()
                    .map(|offer| MarketplaceOfferDto {
                        offer_id: offer.offer_id,
                        village_id: offer.village_id,
                        village_name: offer.village_name,
                        position: PositionDto {
                            x: offer.position.x,
                            y: offer.position.y,
                        },
                        offer_resources: resource_group_to_dto(&offer.offer_resources),
                        seek_resources: resource_group_to_dto(&offer.seek_resources),
                        merchants_required: offer.merchants_required,
                        created_at: offer.created_at,
                    })
                    .collect();

                let global_offers = prepare_global_offers(&marketplace_data)
                    .into_iter()
                    .map(|offer| MarketplaceOfferDto {
                        offer_id: offer.offer_id,
                        village_id: offer.village_id,
                        village_name: offer.village_name,
                        position: PositionDto {
                            x: offer.position.x,
                            y: offer.position.y,
                        },
                        offer_resources: resource_group_to_dto(&offer.offer_resources),
                        seek_resources: resource_group_to_dto(&offer.seek_resources),
                        merchants_required: offer.merchants_required,
                        created_at: offer.created_at,
                    })
                    .collect();

                let outgoing_movements = prepare_merchant_movements(
                    &marketplace_data.outgoing_merchants,
                    &marketplace_data.village_info,
                    MerchantMovementDirection::Outgoing,
                );
                let incoming_movements = prepare_merchant_movements(
                    &marketplace_data.incoming_merchants,
                    &marketplace_data.village_info,
                    MerchantMovementDirection::Incoming,
                );
                let mut merchant_movements: Vec<_> = outgoing_movements
                    .into_iter()
                    .chain(incoming_movements.into_iter())
                    .map(|movement| MerchantMovementDto {
                        job_id: movement.job_id,
                        direction: match movement.direction {
                            MerchantMovementDirection::Outgoing => {
                                MerchantMovementDirectionDto::Outgoing
                            }
                            MerchantMovementDirection::Incoming => {
                                MerchantMovementDirectionDto::Incoming
                            }
                        },
                        kind: match movement.kind {
                            parabellum_app::ports::queries::MerchantMovementKind::Going => {
                                MerchantMovementKindDto::Going
                            }
                            parabellum_app::ports::queries::MerchantMovementKind::Return => {
                                MerchantMovementKindDto::Return
                            }
                        },
                        origin_name: movement.origin_name,
                        origin_position: movement
                            .origin_position
                            .map(|pos| PositionDto { x: pos.x, y: pos.y }),
                        destination_name: movement.destination_name,
                        destination_position: movement
                            .destination_position
                            .map(|pos| PositionDto { x: pos.x, y: pos.y }),
                        resources: resource_group_to_dto(&movement.resources),
                        merchants_used: movement.merchants_used,
                        arrives_at: movement.arrives_at,
                    })
                    .collect();
                merchant_movements.sort_by_key(|movement| movement.arrives_at);

                BuildingDetailDto {
                    slot_id,
                    village_id: user.village.id,
                    building_name: building_key(&slot.building.name),
                    building_type: BuildingTypeDto::Marketplace,
                    current_level,
                    population: slot.building.population,
                    current_upkeep: slot.building.cost().upkeep,
                    next_level,
                    next_upkeep,
                    time_secs,
                    queue_full,
                    at_max_level,
                    next_value,
                    cost: resource_group_to_dto(&cost),
                    stored_resources: resource_group_to_dto(&stored),
                    empty_slot: None,
                    training: None,
                    expansion: None,
                    academy: None,
                    smithy: None,
                    marketplace: Some(MarketplaceDetailDto {
                        available_merchants: user.village.available_merchants(),
                        total_merchants: user.village.total_merchants,
                        own_offers,
                        global_offers,
                        merchant_movements,
                    }),
                    rally_point: None,
                    description_paragraphs: building_description_paragraphs(&slot.building.name),
                }
            }
            BuildingTypeDto::RallyPoint => {
                let movements = state
                    .game_app
                    .get_village_troop_movements(user.village.id)
                    .await
                    .map_err(|err| {
                        map_application_error("unable_to_load_village_troop_movements", err)
                    })?;
                let army_state = state
                    .game_app
                    .get_village_army_state_view(user.village.id)
                    .await
                    .map_err(|err| {
                        map_application_error("unable_to_load_village_army_state", err)
                    })?;
                let village_info = fetch_village_info_for_rally_point(&state, &army_state)
                    .await
                    .map_err(|err| {
                        map_application_error("unable_to_load_rally_village_info", err)
                    })?;
                let cards = prepare_rally_point_cards(
                    user.village.id,
                    &user.village.name,
                    &user.village.position,
                    &user.village.tribe,
                    &army_state,
                    &movements,
                    &village_info,
                )
                .into_iter()
                .map(|card| {
                    let (action, action_id) = match card.action_button {
                        Some(crate::view_helpers::ArmyAction::Recall { army_id }) => {
                            (Some(RallyActionDto::Recall), Some(army_id))
                        }
                        Some(crate::view_helpers::ArmyAction::Release { army_id }) => {
                            (Some(RallyActionDto::Release), Some(army_id))
                        }
                        None => (None, None),
                    };

                    RallyCardDto {
                        village_id: card.village_id,
                        village_name: card.village_name,
                        position: card.position.map(|pos| PositionDto { x: pos.x, y: pos.y }),
                        tribe: format!("{:?}", card.tribe),
                        units: card.units.units().to_vec(),
                        upkeep: troop_upkeep_for_tribe(&card.tribe, card.units.units()),
                        category: match card.category {
                            crate::view_helpers::ArmyCategory::Stationed => {
                                RallyCardCategoryDto::Stationed
                            }
                            crate::view_helpers::ArmyCategory::Reinforcement => {
                                RallyCardCategoryDto::Reinforcement
                            }
                            crate::view_helpers::ArmyCategory::Deployed => {
                                RallyCardCategoryDto::Deployed
                            }
                            crate::view_helpers::ArmyCategory::Incoming => {
                                RallyCardCategoryDto::Incoming
                            }
                            crate::view_helpers::ArmyCategory::Outgoing => {
                                RallyCardCategoryDto::Outgoing
                            }
                        },
                        movement_kind: card.movement_kind.map(|kind| match kind {
                            crate::view_helpers::MovementKind::Attack => {
                                RallyMovementKindDto::Attack
                            }
                            crate::view_helpers::MovementKind::Raid => RallyMovementKindDto::Raid,
                            crate::view_helpers::MovementKind::Scout => {
                                RallyMovementKindDto::Scout
                            }
                            crate::view_helpers::MovementKind::Reinforcement => {
                                RallyMovementKindDto::Reinforcement
                            }
                            crate::view_helpers::MovementKind::Return => {
                                RallyMovementKindDto::Return
                            }
                            crate::view_helpers::MovementKind::FoundVillage => {
                                RallyMovementKindDto::FoundVillage
                            }
                        }),
                        arrives_at: card.arrives_at,
                        bounty: card.bounty.as_ref().map(resource_group_to_dto),
                        action,
                        action_id,
                    }
                })
                .collect();

                let available_units = army_state
                    .home_army
                    .as_ref()
                    .map(|army| army.units().clone())
                    .unwrap_or_default();
                let sendable_units = user
                    .village
                    .tribe
                    .units()
                    .iter()
                    .enumerate()
                    .map(|(idx, unit)| RallySendableUnitDto {
                        unit_idx: idx,
                        name: unit_key(&unit.name),
                        available: available_units.get(idx),
                        is_researched: user.village.academy_research().get(idx)
                            || unit.research_cost.time == 0,
                    })
                    .collect();

                BuildingDetailDto {
                    slot_id,
                    village_id: user.village.id,
                    building_name: building_key(&slot.building.name),
                    building_type: BuildingTypeDto::RallyPoint,
                    current_level,
                    population: slot.building.population,
                    current_upkeep: slot.building.cost().upkeep,
                    next_level,
                    next_upkeep,
                    time_secs,
                    queue_full,
                    at_max_level,
                    next_value,
                    cost: resource_group_to_dto(&cost),
                    stored_resources: resource_group_to_dto(&stored),
                    empty_slot: None,
                    training: None,
                    expansion: None,
                    academy: None,
                    smithy: None,
                    marketplace: None,
                    rally_point: Some(RallyPointDetailDto {
                        cards,
                        sendable_units,
                    }),
                    description_paragraphs: building_description_paragraphs(&slot.building.name),
                }
            }
        }
    } else {
        let queued = queues.building.iter().find(|item| item.slot_id == slot_id);
        let has_queue_for_slot = queued.is_some();
        let (buildable_buildings, locked_buildings) = if has_queue_for_slot {
            (vec![], vec![])
        } else {
            build_options_for_slot(&user.village, slot_id, &queues.building, state.server_speed)
        };
        let queued_target_level = queued.map(|item| item.target_level);
        let queued_next_level = queued_target_level.map(|level| level.saturating_add(1));
        let queued_can_upgrade = queued.and_then(|item| {
            get_building_data(&item.building_name)
                .ok()
                .map(|data| item.target_level < data.rules.max_level)
        });
        let queued_upgrade_preview = queued.and_then(|item| {
            let current_level = item.target_level;
            let building_name = item.building_name.clone();
            let template = Building::new(building_name.clone(), state.server_speed);
            let current_building = template.at_level(current_level, state.server_speed).ok();
            let current_upkeep = current_building
                .as_ref()
                .map(|b| b.cost().upkeep)
                .unwrap_or(template.cost().upkeep);
            let max_level = get_building_data(&building_name)
                .map(|data| data.rules.max_level)
                .unwrap_or(current_level);
            let at_max_level = current_level >= max_level;
            let next_level = current_level.saturating_add(1).min(max_level);
            let main_building_level = user.village.main_building_level();
            let next_building = if at_max_level {
                None
            } else {
                template.at_level(next_level, state.server_speed).ok()
            };
            let (next_upkeep, time_secs, cost, next_value) = if let Some(ref upgraded) =
                next_building
            {
                let computed = upgraded.cost();
                (
                    computed.upkeep,
                    upgraded.calculate_build_time_secs(&state.server_speed, &main_building_level),
                    resource_group_to_dto(&computed.resources),
                    Some(format_next_value(building_name.clone(), upgraded.value)),
                )
            } else {
                (
                    current_upkeep,
                    0,
                    resource_group_to_dto(&ResourceGroup::new(0, 0, 0, 0)),
                    None,
                )
            };

            Some(QueuedUpgradePreviewDto {
                building_name: building_key(&building_name),
                current_level,
                next_level,
                current_upkeep,
                next_upkeep,
                time_secs,
                at_max_level,
                next_value,
                cost,
            })
        });

        BuildingDetailDto {
            slot_id,
            village_id: user.village.id,
            building_name: "EmptySlot".to_string(),
            building_type: BuildingTypeDto::Empty,
            current_level: 0,
            population: 0,
            current_upkeep: 0,
            next_level: 1,
            next_upkeep: 0,
            time_secs: 0,
            queue_full,
            at_max_level: false,
            next_value: None,
            cost: resource_group_to_dto(&ResourceGroup::new(0, 0, 0, 0)),
            stored_resources: resource_group_to_dto(&stored),
            empty_slot: Some(EmptySlotDetailDto {
                buildable_buildings,
                locked_buildings,
                has_queue_for_slot,
                queued_building_name: queued.map(|item| building_key(&item.building_name)),
                queued_target_level,
                queued_next_level,
                queued_can_upgrade,
                queued_upgrade_preview,
            }),
            training: None,
            expansion: None,
            academy: None,
            smithy: None,
            marketplace: None,
            rally_point: None,
            description_paragraphs: vec![],
        }
    };

    Ok(Json(BuildingPageResponse {
        server_time: Utc::now().timestamp(),
        village: village_summary(&user.village),
        villages: village_list(&user),
        detail,
    }))
}

async fn fetch_village_info_for_rally_point(
    state: &AppState,
    army_state: &VillageArmyStateView,
) -> Result<HashMap<u32, VillageInfo>, ApplicationError> {
    let mut village_ids = std::collections::HashSet::new();

    for army in &army_state.deployed_armies {
        if let Some(dest_id) = army.current_map_field_id {
            village_ids.insert(dest_id);
        }
    }

    for reinforcement in &army_state.reinforcements {
        village_ids.insert(reinforcement.village_id);
    }

    if village_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let ids: Vec<u32> = village_ids.into_iter().collect();

    state.game_app.get_village_info_by_ids(ids).await
}

fn resource_group_to_dto(resource: &ResourceGroup) -> ResourceAmountsDto {
    ResourceAmountsDto {
        lumber: resource.lumber(),
        clay: resource.clay(),
        iron: resource.iron(),
        crop: resource.crop(),
    }
}

fn building_key(name: &BuildingName) -> String {
    format!("{name:?}")
}

fn unit_key(name: &UnitName) -> String {
    format!("{name:?}")
}

fn format_next_value(name: BuildingName, value: u32) -> String {
    match name {
        BuildingName::Barracks
        | BuildingName::GreatBarracks
        | BuildingName::Stable
        | BuildingName::GreatStable
        | BuildingName::Workshop
        | BuildingName::GreatWorkshop => format!("{}%", (value as f32 / 10.0) as u32),
        _ => format!("{value}"),
    }
}

fn build_options_for_slot(
    village: &parabellum_game::models::village::Village,
    slot_id: u8,
    queue: &[BuildingQueueItem],
    server_speed: i8,
) -> (Vec<BuildOptionDto>, Vec<BuildOptionDto>) {
    let mut buildable = Vec::new();
    let mut locked = Vec::new();
    let main_building_level = village.main_building_level();

    for name in village.candidate_buildings_for_slot(slot_id) {
        if building_blocked_by_queue(&name, queue) {
            continue;
        }

        let building = Building::new(name.clone(), server_speed);
        let validation_ok = village.validate_building_construction(&building).is_ok();
        let missing_requirements = missing_building_requirements(village, &name);

        if !validation_ok && missing_requirements.is_empty() {
            continue;
        }

        let cost = building.cost();
        let time_secs = building.calculate_build_time_secs(&server_speed, &main_building_level);
        let option = BuildOptionDto {
            building_name: building_key(&name),
            cost: resource_group_to_dto(&cost.resources),
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

fn building_blocked_by_queue(name: &BuildingName, queue: &[BuildingQueueItem]) -> bool {
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

fn missing_building_requirements(
    village: &parabellum_game::models::village::Village,
    name: &BuildingName,
) -> Vec<RequirementDto> {
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
                Some(RequirementDto {
                    building_name: building_key(&req.0),
                    required_level: req.1,
                })
            }
        })
        .collect()
}

fn training_options_for_group(
    village: &parabellum_game::models::village::Village,
    server_speed: i8,
    building: &VillageBuilding,
    expected_buildings: &[BuildingName],
    group: UnitGroup,
    training_queue: &[TrainingQueueItem],
    foundation_free_slots: Option<u8>,
    moving_expansion_units: (u32, u32),
) -> Vec<TrainingUnitOptionDto> {
    if !expected_buildings.contains(&building.building.name) {
        return vec![];
    }

    let training_multiplier = building.building.value as f64 / 1000.0;
    let available_units = village.available_units_for_training(group);
    let tribe = village.tribe.clone();

    let chiefs_at_home = village.count_chiefs_at_home();
    let settlers_at_home = village.count_settlers_at_home();
    let chiefs_deployed: u32 = village
        .deployed_armies()
        .iter()
        .map(|army| army.units().get(8))
        .sum();
    let settlers_deployed: u32 = village
        .deployed_armies()
        .iter()
        .map(|army| army.units().get(9))
        .sum();
    let mut chiefs_queued = 0u32;
    let mut settlers_queued = 0u32;
    for item in training_queue {
        let qty = item.quantity.max(0) as u32;
        if matches!(item.unit, UnitName::Chief | UnitName::Senator | UnitName::Chieftain) {
            chiefs_queued = chiefs_queued.saturating_add(qty);
        } else if matches!(item.unit, UnitName::Settler) {
            settlers_queued = settlers_queued.saturating_add(qty);
        }
    }
    let chiefs_total = chiefs_at_home + chiefs_deployed + chiefs_queued + moving_expansion_units.0;
    let settlers_total =
        settlers_at_home + settlers_deployed + settlers_queued + moving_expansion_units.1;
    let available_slots = foundation_free_slots.unwrap_or_else(|| village.max_foundation_slots());

    available_units
        .into_iter()
        .filter_map(|unit| {
            if matches!(unit.role, UnitRole::Chief | UnitRole::Settler) {
                let committed_this_unit = if matches!(unit.role, UnitRole::Chief) {
                    chiefs_total
                } else {
                    settlers_total
                };
                let max_trainable = parabellum_game::models::village::Village::max_expansion_unit_trainable(
                    unit.role.clone(),
                    available_slots,
                    chiefs_total,
                    settlers_total,
                    committed_this_unit,
                );
                if max_trainable == 0 {
                    return None;
                }
            }
            let unit_idx = tribe.get_unit_idx_by_name(&unit.name)? as u8;
            let base_time_per_unit = unit.cost.time as f64 / server_speed as f64;
            let time_per_unit = (base_time_per_unit * training_multiplier).floor().max(1.0) as u32;

            Some(TrainingUnitOptionDto {
                unit_idx,
                name: unit_key(&unit.name),
                cost: resource_group_to_dto(&unit.cost.resources),
                upkeep: unit.cost.upkeep,
                attack: unit.attack,
                defense_infantry: unit.defense_infantry,
                defense_cavalry: unit.defense_cavalry,
                speed: unit.speed,
                capacity: unit.capacity,
                time_secs: time_per_unit,
            })
        })
        .collect()
}

fn troop_upkeep_for_tribe(tribe: &Tribe, units: &[u32; 10]) -> u32 {
    tribe
        .units()
        .iter()
        .enumerate()
        .map(|(idx, unit)| unit.cost.upkeep.saturating_mul(*units.get(idx).unwrap_or(&0)))
        .sum()
}

fn training_queue_for_slot(slot_id: u8, queue: &[TrainingQueueItem]) -> Vec<TrainingQueueItemDto> {
    queue
        .iter()
        .filter(|item| item.slot_id == slot_id)
        .map(|item| TrainingQueueItemDto {
            quantity: item.quantity.max(0) as u32,
            unit_name: unit_key(&item.unit),
            time_per_unit: item.time_per_unit.max(0) as u32,
            finishes_at: item.finishes_at,
        })
        .collect()
}

fn academy_options_for_village(
    village: &parabellum_game::models::village::Village,
    server_speed: i8,
    queued_jobs: &[AcademyQueueItem],
) -> (
    Vec<AcademyResearchOptionDto>,
    Vec<AcademyResearchOptionDto>,
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
            researched.push(unit_key(&unit.name));
        } else {
            let option = AcademyResearchOptionDto {
                unit_name: unit_key(&unit.name),
                cost: resource_group_to_dto(&unit.research_cost.resources),
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

fn academy_queue_for_slot(queue: &[AcademyQueueItem]) -> Vec<AcademyQueueItemDto> {
    queue
        .iter()
        .map(|item| AcademyQueueItemDto {
            unit_name: unit_key(&item.unit),
            finishes_at: item.finishes_at,
            is_processing: item.status == ScheduledActionStatus::Processing,
        })
        .collect()
}

fn missing_unit_requirements(
    village: &parabellum_game::models::village::Village,
    requirements: &[BuildingRequirement],
) -> Vec<RequirementDto> {
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
                Some(RequirementDto {
                    building_name: building_key(&req.0),
                    required_level: req.1,
                })
            }
        })
        .collect()
}

fn smithy_options_for_village(
    village: &parabellum_game::models::village::Village,
    smithy_building: &VillageBuilding,
    server_speed: i8,
    queue: &[SmithyQueueItem],
    queue_full: bool,
) -> Vec<SmithyUpgradeOptionDto> {
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

        options.push(SmithyUpgradeOptionDto {
            unit_name: unit_key(&unit.name),
            current_level,
            max_level: smithy_level_cap,
            cost: resource_group_to_dto(&cost),
            time_secs,
            can_upgrade,
        });
    }

    options
}

fn smithy_queue_counts(queue: &[SmithyQueueItem]) -> HashMap<UnitName, u8> {
    let mut counts = HashMap::new();
    for job in queue {
        *counts.entry(job.unit.clone()).or_insert(0) += 1;
    }
    counts
}

fn smithy_queue_for_slot(queue: &[SmithyQueueItem]) -> Vec<SmithyQueueItemDto> {
    let mut unit_counts: HashMap<UnitName, u8> = HashMap::new();

    queue
        .iter()
        .map(|item| {
            let count = unit_counts.entry(item.unit.clone()).or_insert(0);
            *count += 1;

            SmithyQueueItemDto {
                unit_name: unit_key(&item.unit),
                target_level: *count,
                finishes_at: item.finishes_at,
                is_processing: item.status == ScheduledActionStatus::Processing,
            }
        })
        .collect()
}
