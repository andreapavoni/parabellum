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
    read_models::VillageReference,
    villages::models::{BuildingWorkflowKind, ScheduledActionStatus},
    villages::read_models::{
        AcademyQueueItem, BuildingQueueItem, MarketplaceData, MerchantMovement,
        MerchantMovementKind, SmithyQueueItem, TrainingQueueItem, TrapQueueItem, TroopMovementType,
        VillageArmyStateView, VillageTroopMovements,
    },
};
use parabellum_game::models::{
    buildings::{Building, get_building_data},
    marketplace::MarketplaceOffer,
    smithy::smithy_upgrade_cost_for_unit,
    trapper::{TRAP_BUILD_TIME_SECS, TRAP_COST, Trapper},
    village::VillageBuilding,
};
use parabellum_types::{
    army::{TroopSet, UnitGroup, UnitName, UnitRole},
    buildings::{BuildingName, BuildingRequirement},
    common::ResourceGroup,
    errors::ApplicationError,
    map::Position,
    tribe::Tribe,
};

use crate::{
    api::{dto::ResourceAmountsDto, errors::ApiError},
    http::AppState,
};

use super::authenticated_user;
use super::error_mapping::map_application_error;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Response payload for `GET /api/v1/buildings/{slot_id}`.
pub struct BuildingPageResponse {
    pub server_time: i64,
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
    pub current_value: Option<u32>,
    pub next_value: Option<u32>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trapper: Option<TrapperDetailDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_building: Option<MainBuildingDetailDto>,
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
    pub loyalty: u8,
    pub village_culture_points_production: u32,
    pub account_culture_points_production: u32,
    pub account_culture_points: u32,
    pub next_cp_required: u32,
    pub max_foundation_slots: u8,
    pub child_villages_count: u32,
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
    pub current_value: Option<u32>,
    pub next_value: Option<u32>,
    pub cost: ResourceAmountsDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildOptionDto {
    pub building_name: String,
    pub cost: ResourceAmountsDto,
    pub next_upkeep: u32,
    pub upkeep: u32,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_trainable: Option<u32>,
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
    pub next_level: u8,
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
    pub merchant_capacity: u32,
    pub merchant_speed: u32,
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
    pub distance: u32,
    pub travel_time_seconds: u32,
    pub offer_resources: ResourceAmountsDto,
    pub seek_resources: ResourceAmountsDto,
    pub merchants_required: u8,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MerchantMovementDirectionDto {
    Outgoing,
    Incoming,
}

#[derive(Debug, Clone, Copy, Serialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trapper: Option<TrapperDetailDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrapperDetailDto {
    pub capacity: u32,
    pub active: u32,
    pub occupied: u32,
    pub broken: u32,
    pub queued: u32,
    pub buildable: u32,
    pub queue: Vec<TrapQueueItemDto>,
    pub cost_per_trap: ResourceAmountsDto,
    pub time_per_trap_seconds: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrapQueueItemDto {
    pub quantity: u32,
    pub time_per_trap: u32,
    pub finishes_at: chrono::DateTime<chrono::Utc>,
    pub is_processing: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MainBuildingDetailDto {
    pub can_downgrade: bool,
    pub queue_full: bool,
    pub options: Vec<BuildingDowngradeOptionDto>,
    pub queue: Vec<MainBuildingDowngradeQueueItemDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingDowngradeOptionDto {
    pub slot_id: u8,
    pub building_name: String,
    pub current_level: u8,
    pub next_level: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MainBuildingDowngradeQueueItemDto {
    pub action_id: String,
    pub slot_id: u8,
    pub building_name: String,
    pub target_level: u8,
    pub finishes_at: chrono::DateTime<chrono::Utc>,
    pub time_seconds: u32,
    pub is_processing: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RallyCardCategoryDto {
    Stationed,
    Reinforcement,
    Deployed,
    Trapped,
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RallyMovementKindDto {
    Attack,
    Raid,
    Scout,
    Reinforcement,
    Return,
    FoundVillage,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RallyActionDto {
    Recall,
    Release,
    Cancel,
    ReleaseTrapped,
    DisbandTrapped,
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
    pub has_hero: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upkeep: Option<u32>,
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum MovementKind {
    Attack,
    Raid,
    Scout,
    Reinforcement,
    Return,
    FoundVillage,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ArmyCategory {
    Stationed,
    Reinforcement,
    Deployed,
    Trapped,
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, PartialEq)]
enum ArmyAction {
    Recall { army_id: String },
    Release { army_id: String },
    Cancel { movement_id: String },
    ReleaseTrapped { army_id: String },
    DisbandTrapped { army_id: String },
}

#[derive(Debug, Clone, PartialEq)]
struct ArmyCardData {
    village_id: u32,
    village_name: Option<String>,
    position: Option<Position>,
    units: TroopSet,
    has_hero: bool,
    tribe: Tribe,
    category: ArmyCategory,
    movement_kind: Option<MovementKind>,
    arrives_at: Option<chrono::DateTime<chrono::Utc>>,
    bounty: Option<ResourceGroup>,
    action_button: Option<ArmyAction>,
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
    let village_model = state
        .game_app
        .get_village_state(user.village.id)
        .await
        .map_err(|err| map_application_error("unable_to_load_village", err))?;
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
    let building_queue_full = queues.building.len() >= building_queue_capacity;
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
            .filter(|item| {
                item.slot_id == slot_id
                    && matches!(
                        item.kind,
                        BuildingWorkflowKind::Add | BuildingWorkflowKind::Upgrade
                    )
            })
            .count() as u8;
        let has_pending_downgrade_for_slot = queues.building.iter().any(|item| {
            item.slot_id == slot_id && matches!(item.kind, BuildingWorkflowKind::Downgrade)
        });
        let queue_full = building_queue_full || has_pending_downgrade_for_slot;
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

        let current_value = if slot.building.value == 0 {
            None
        } else {
            Some(slot.building.value)
        };
        let next_value = upgrade_info.as_ref().map(|upgraded| upgraded.value);
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
        let main_building_detail = if matches!(slot.building.name, BuildingName::MainBuilding) {
            Some(main_building_detail_for_village(
                &user.village,
                &queues.building,
            ))
        } else {
            None
        };

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
                current_value,
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
                trapper: if slot.building.name == BuildingName::Trapper {
                    let army_state = state
                        .game_app
                        .get_village_army_state_view(user.village.id)
                        .await
                        .map_err(|err| {
                            map_application_error("unable_to_load_village_army_state", err)
                        })?;
                    trapper_detail_for_village(&village_model, Some(&army_state), &queues.traps)
                } else {
                    None
                },
                main_building: main_building_detail,
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
                    None,
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
                    current_value,
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
                    trapper: None,
                    main_building: None,
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
                let army_state = state
                    .game_app
                    .get_village_army_state_view(user.village.id)
                    .await
                    .map_err(|err| {
                        map_application_error("unable_to_load_village_army_state", err)
                    })?;
                let training_units = training_options_for_group(
                    &user.village,
                    state.server_speed,
                    &slot,
                    &[BuildingName::Residence, BuildingName::Palace],
                    UnitGroup::Expansion,
                    &queues.training,
                    Some(available_slots),
                    (chiefs_moving, settlers_moving),
                    Some(&army_state),
                );
                let training_queue = training_queue_for_slot(slot_id, &queues.training);
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
                    current_value,
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
                        loyalty: user.village.loyalty(),
                        village_culture_points_production,
                        account_culture_points_production,
                        account_culture_points,
                        next_cp_required,
                        max_foundation_slots: max_slots,
                        child_villages_count,
                    }),
                    academy: None,
                    smithy: None,
                    marketplace: None,
                    rally_point: None,
                    trapper: None,
                    main_building: None,
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
                    current_value,
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
                    trapper: None,
                    main_building: None,
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
                    current_value,
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
                    trapper: None,
                    main_building: None,
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

                let base_merchant_speed = user.village.tribe.merchant_stats().speed;
                let effective_merchant_speed =
                    base_merchant_speed.saturating_mul(state.server_speed.max(1) as u8);

                let own_offers = marketplace_data
                    .own_offers
                    .iter()
                    .map(|offer| {
                        marketplace_offer_to_dto(
                            &marketplace_data,
                            offer,
                            &user.village.position,
                            base_merchant_speed.max(1),
                            state.world_size,
                            state.server_speed as u8,
                        )
                    })
                    .collect();

                let global_offers = marketplace_data
                    .global_offers
                    .iter()
                    .map(|offer| {
                        marketplace_offer_to_dto(
                            &marketplace_data,
                            offer,
                            &user.village.position,
                            base_merchant_speed.max(1),
                            state.world_size,
                            state.server_speed as u8,
                        )
                    })
                    .collect();

                let pending_outgoing_goings: Vec<&MerchantMovement> = marketplace_data
                    .outgoing_merchants
                    .iter()
                    .filter(|movement| matches!(movement.kind, MerchantMovementKind::Going))
                    .collect();

                let mut merchant_movements: Vec<_> = Vec::new();
                for movement in &marketplace_data.outgoing_merchants {
                    let hide_prescheduled_return =
                        matches!(movement.kind, MerchantMovementKind::Return)
                            && paired_outgoing_going_exists(
                                &marketplace_data,
                                movement,
                                &pending_outgoing_goings,
                            );
                    if !hide_prescheduled_return {
                        merchant_movements.push(merchant_movement_to_dto(
                            &marketplace_data,
                            movement,
                            MerchantMovementDirectionDto::Outgoing,
                        ));
                    }
                }
                merchant_movements.extend(marketplace_data.incoming_merchants.iter().map(
                    |movement| {
                        merchant_movement_to_dto(
                            &marketplace_data,
                            movement,
                            MerchantMovementDirectionDto::Incoming,
                        )
                    },
                ));
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
                    current_value,
                    next_value,
                    cost: resource_group_to_dto(&cost),
                    stored_resources: resource_group_to_dto(&stored),
                    empty_slot: None,
                    training: None,
                    expansion: None,
                    academy: None,
                    smithy: None,
                    marketplace: Some(MarketplaceDetailDto {
                        // Marketplace stats are displayed as effective values for current server speed.
                        // Tribe base values are 1x; gameplay speed multiplies both capacity and speed.
                        merchant_capacity: user
                            .village
                            .tribe
                            .merchant_stats()
                            .capacity
                            .saturating_mul(state.server_speed.max(1) as u32),
                        merchant_speed: effective_merchant_speed as u32,
                        available_merchants: user.village.available_merchants(),
                        total_merchants: user.village.total_merchants,
                        own_offers,
                        global_offers,
                        merchant_movements,
                    }),
                    rally_point: None,
                    trapper: None,
                    main_building: None,
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
                let village_references =
                    fetch_village_references_for_rally_point(&state, &army_state)
                        .await
                        .map_err(|err| {
                            map_application_error("unable_to_load_rally_village_references", err)
                        })?;
                let cancelable_movement_ids = state
                    .game_app
                    .list_cancelable_outgoing_movement_ids(user.village.id)
                    .await
                    .map_err(|err| {
                        map_application_error("unable_to_load_cancelable_movements", err)
                    })?;
                let cards = prepare_rally_point_cards(
                    user.village.id,
                    &user.village.name,
                    &user.village.position,
                    &user.village.tribe,
                    &army_state,
                    &movements,
                    &village_references,
                    &cancelable_movement_ids,
                )
                .into_iter()
                .map(|card| {
                    let upkeep = troop_upkeep_for_rally_card(&user.village, &card);
                    let (action, action_id) = match card.action_button {
                        Some(ArmyAction::Recall { army_id }) => {
                            (Some(RallyActionDto::Recall), Some(army_id))
                        }
                        Some(ArmyAction::Release { army_id }) => {
                            (Some(RallyActionDto::Release), Some(army_id))
                        }
                        Some(ArmyAction::Cancel { movement_id }) => {
                            (Some(RallyActionDto::Cancel), Some(movement_id))
                        }
                        Some(ArmyAction::ReleaseTrapped { army_id }) => {
                            (Some(RallyActionDto::ReleaseTrapped), Some(army_id))
                        }
                        Some(ArmyAction::DisbandTrapped { army_id }) => {
                            (Some(RallyActionDto::DisbandTrapped), Some(army_id))
                        }
                        None => (None, None),
                    };

                    RallyCardDto {
                        village_id: card.village_id,
                        village_name: card.village_name,
                        position: card.position.map(|pos| PositionDto { x: pos.x, y: pos.y }),
                        tribe: format!("{:?}", card.tribe),
                        units: card.units.units().to_vec(),
                        has_hero: card.has_hero,
                        upkeep,
                        category: match card.category {
                            ArmyCategory::Stationed => RallyCardCategoryDto::Stationed,
                            ArmyCategory::Reinforcement => RallyCardCategoryDto::Reinforcement,
                            ArmyCategory::Deployed => RallyCardCategoryDto::Deployed,
                            ArmyCategory::Trapped => RallyCardCategoryDto::Trapped,
                            ArmyCategory::Incoming => RallyCardCategoryDto::Incoming,
                            ArmyCategory::Outgoing => RallyCardCategoryDto::Outgoing,
                        },
                        movement_kind: card.movement_kind.map(|kind| match kind {
                            MovementKind::Attack => RallyMovementKindDto::Attack,
                            MovementKind::Raid => RallyMovementKindDto::Raid,
                            MovementKind::Scout => RallyMovementKindDto::Scout,
                            MovementKind::Reinforcement => RallyMovementKindDto::Reinforcement,
                            MovementKind::Return => RallyMovementKindDto::Return,
                            MovementKind::FoundVillage => RallyMovementKindDto::FoundVillage,
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
                    current_value,
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
                        trapper: trapper_detail_for_village(
                            &village_model,
                            Some(&army_state),
                            &queues.traps,
                        ),
                    }),
                    trapper: None,
                    main_building: None,
                }
            }
        }
    } else {
        let queued_for_slot: Vec<&BuildingQueueItem> = queues
            .building
            .iter()
            .filter(|item| item.slot_id == slot_id)
            .collect();
        let queued = queued_for_slot.last().copied();
        let has_queue_for_slot = !queued_for_slot.is_empty();
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
        let queued_upgrade_preview = queued.map(|item| {
            let current_level = item.target_level;
            let building_name = item.building_name.clone();
            let template = Building::new(building_name.clone(), state.server_speed);
            let current_building = template.at_level(current_level, state.server_speed).ok();
            let current_upkeep = current_building
                .as_ref()
                .map(|b| b.cost().upkeep)
                .unwrap_or(template.cost().upkeep);
            let current_value = current_building
                .as_ref()
                .and_then(|building| (building.value > 0).then_some(building.value));
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
                    Some(upgraded.value),
                )
            } else {
                (
                    current_upkeep,
                    0,
                    resource_group_to_dto(&ResourceGroup::new(0, 0, 0, 0)),
                    None,
                )
            };

            QueuedUpgradePreviewDto {
                building_name: building_key(&building_name),
                current_level,
                next_level,
                current_upkeep,
                next_upkeep,
                time_secs,
                at_max_level,
                current_value,
                next_value,
                cost,
            }
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
            queue_full: building_queue_full,
            at_max_level: false,
            current_value: None,
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
            trapper: None,
            main_building: None,
        }
    };

    Ok(Json(BuildingPageResponse {
        server_time: Utc::now().timestamp(),
        detail,
    }))
}

async fn fetch_village_references_for_rally_point(
    state: &AppState,
    army_state: &VillageArmyStateView,
) -> Result<HashMap<u32, VillageReference>, ApplicationError> {
    let mut village_ids = std::collections::HashSet::new();

    for army in &army_state.deployed_armies {
        if let Some(dest_id) = army.current_map_field_id {
            village_ids.insert(dest_id);
        }
    }

    for reinforcement in &army_state.reinforcements {
        village_ids.insert(reinforcement.village_id);
    }

    for trapped in &army_state.trapped_here {
        village_ids.insert(trapped.village_id);
    }

    for trapped in &army_state.trapped_away {
        if let Some(dest_id) = trapped.current_map_field_id {
            village_ids.insert(dest_id);
        }
    }

    if village_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let ids: Vec<u32> = village_ids.into_iter().collect();

    state.game_app.get_village_references(ids).await
}

fn prepare_rally_point_cards(
    village_id: u32,
    village_name: &str,
    village_position: &Position,
    village_tribe: &Tribe,
    armies: &VillageArmyStateView,
    movements: &VillageTroopMovements,
    village_references: &HashMap<u32, VillageReference>,
    cancelable_movement_ids: &std::collections::HashSet<uuid::Uuid>,
) -> Vec<ArmyCardData> {
    let mut cards = Vec::new();

    if let Some(army) = &armies.home_army {
        cards.push(ArmyCardData {
            village_id,
            village_name: Some(village_name.to_string()),
            position: Some(village_position.clone()),
            units: army.units().clone(),
            has_hero: army.hero().is_some(),
            tribe: village_tribe.clone(),
            category: ArmyCategory::Stationed,
            movement_kind: None,
            arrives_at: None,
            bounty: None,
            action_button: None,
        });
    }

    for army in &armies.deployed_armies {
        let destination_id = army.current_map_field_id.unwrap_or(village_id);
        let (destination_name, destination_position) = village_references
            .get(&destination_id)
            .map(|info| (Some(info.name.clone()), Some(info.position.clone())))
            .unwrap_or_else(|| (Some(format!("Village #{}", destination_id)), None));

        cards.push(ArmyCardData {
            village_id: destination_id,
            village_name: destination_name,
            position: destination_position,
            units: army.units().clone(),
            has_hero: army.hero().is_some(),
            tribe: army.tribe.clone(),
            category: ArmyCategory::Deployed,
            movement_kind: None,
            arrives_at: None,
            bounty: None,
            action_button: Some(ArmyAction::Recall {
                army_id: army.id.to_string(),
            }),
        });
    }

    for reinforcement in &armies.reinforcements {
        let origin_id = reinforcement.village_id;
        let (origin_name, origin_position) = village_references
            .get(&origin_id)
            .map(|info| (Some(info.name.clone()), Some(info.position.clone())))
            .unwrap_or_else(|| (Some(format!("Village #{}", origin_id)), None));

        cards.push(ArmyCardData {
            village_id: origin_id,
            village_name: origin_name,
            position: origin_position,
            units: reinforcement.units().clone(),
            has_hero: reinforcement.hero().is_some(),
            tribe: reinforcement.tribe.clone(),
            category: ArmyCategory::Reinforcement,
            movement_kind: None,
            arrives_at: None,
            bounty: None,
            action_button: Some(ArmyAction::Release {
                army_id: reinforcement.id.to_string(),
            }),
        });
    }

    for trapped in &armies.trapped_here {
        let origin_id = trapped.village_id;
        let (origin_name, origin_position) = village_references
            .get(&origin_id)
            .map(|info| (Some(info.name.clone()), Some(info.position.clone())))
            .unwrap_or_else(|| (Some(format!("Village #{}", origin_id)), None));

        cards.push(ArmyCardData {
            village_id: origin_id,
            village_name: origin_name,
            position: origin_position,
            units: trapped.units().clone(),
            has_hero: trapped.hero().is_some(),
            tribe: trapped.tribe.clone(),
            category: ArmyCategory::Trapped,
            movement_kind: None,
            arrives_at: None,
            bounty: None,
            action_button: Some(ArmyAction::ReleaseTrapped {
                army_id: trapped.id.to_string(),
            }),
        });
    }

    for trapped in &armies.trapped_away {
        let destination_id = trapped.current_map_field_id.unwrap_or(village_id);
        let (destination_name, destination_position) = village_references
            .get(&destination_id)
            .map(|info| (Some(info.name.clone()), Some(info.position.clone())))
            .unwrap_or_else(|| (Some(format!("Village #{}", destination_id)), None));

        cards.push(ArmyCardData {
            village_id: destination_id,
            village_name: destination_name,
            position: destination_position,
            units: trapped.units().clone(),
            has_hero: trapped.hero().is_some(),
            tribe: trapped.tribe.clone(),
            category: ArmyCategory::Trapped,
            movement_kind: None,
            arrives_at: None,
            bounty: None,
            action_button: Some(ArmyAction::DisbandTrapped {
                army_id: trapped.id.to_string(),
            }),
        });
    }

    for movement in &movements.outgoing {
        let action_button = if cancelable_movement_ids.contains(&movement.job_id) {
            Some(ArmyAction::Cancel {
                movement_id: movement.job_id.to_string(),
            })
        } else {
            None
        };

        cards.push(ArmyCardData {
            village_id: movement.target_village_id,
            village_name: movement.target_village_name.clone(),
            position: Some(movement.target_position.clone()),
            units: movement.units.clone(),
            has_hero: movement.has_hero,
            tribe: movement.tribe.clone(),
            category: ArmyCategory::Outgoing,
            movement_kind: Some(movement_kind_to_card_kind(movement.movement_type)),
            arrives_at: Some(movement.arrives_at),
            bounty: movement.bounty.clone(),
            action_button,
        });
    }

    for movement in &movements.incoming {
        cards.push(ArmyCardData {
            village_id: movement.origin_village_id,
            village_name: movement.origin_village_name.clone(),
            position: Some(movement.origin_position.clone()),
            units: movement.units.clone(),
            has_hero: movement.has_hero,
            tribe: movement.tribe.clone(),
            category: ArmyCategory::Incoming,
            movement_kind: Some(movement_kind_to_card_kind(movement.movement_type)),
            arrives_at: Some(movement.arrives_at),
            bounty: movement.bounty.clone(),
            action_button: None,
        });
    }

    cards
}

fn trapper_detail_for_village(
    village: &parabellum_app::villages::models::VillageModel,
    army_state: Option<&VillageArmyStateView>,
    queue: &[TrapQueueItem],
) -> Option<TrapperDetailDto> {
    let occupied = army_state
        .map(|state| {
            state
                .trapped_here
                .iter()
                .map(|army| army.units().immensity())
                .sum()
        })
        .unwrap_or(0);
    let trapper = Trapper::from_buildings(&village.buildings, village.trapper, occupied);
    if trapper.capacity() == 0 {
        return None;
    }

    Some(TrapperDetailDto {
        capacity: trapper.capacity(),
        active: trapper.active_traps(),
        occupied: trapper.occupied_traps(),
        broken: trapper.broken_traps(),
        queued: trapper.queued_traps(),
        buildable: trapper.buildable_traps(),
        queue: trap_queue_to_dto(queue),
        cost_per_trap: resource_group_to_dto(&TRAP_COST),
        time_per_trap_seconds: TRAP_BUILD_TIME_SECS,
    })
}

fn trap_queue_to_dto(queue: &[TrapQueueItem]) -> Vec<TrapQueueItemDto> {
    queue
        .iter()
        .map(|item| TrapQueueItemDto {
            quantity: item.quantity.max(0) as u32,
            time_per_trap: item.time_per_trap.max(0) as u32,
            finishes_at: item.finishes_at,
            is_processing: matches!(item.status, ScheduledActionStatus::Processing),
        })
        .collect()
}

fn movement_kind_to_card_kind(kind: TroopMovementType) -> MovementKind {
    match kind {
        TroopMovementType::Attack => MovementKind::Attack,
        TroopMovementType::Raid => MovementKind::Raid,
        TroopMovementType::Scout => MovementKind::Scout,
        TroopMovementType::Reinforcement => MovementKind::Reinforcement,
        TroopMovementType::Return => MovementKind::Return,
        TroopMovementType::FoundVillage => MovementKind::FoundVillage,
    }
}

fn resource_group_to_dto(resource: &ResourceGroup) -> ResourceAmountsDto {
    ResourceAmountsDto {
        lumber: resource.lumber(),
        clay: resource.clay(),
        iron: resource.iron(),
        crop: resource.crop(),
    }
}

fn main_building_detail_for_village(
    village: &parabellum_game::models::village::Village,
    building_queue: &[BuildingQueueItem],
) -> MainBuildingDetailDto {
    let can_downgrade = village.main_building_level() >= 10;
    let queued_downgrades = building_queue
        .iter()
        .filter(|item| matches!(item.kind, BuildingWorkflowKind::Downgrade))
        .count();
    let queue_full = queued_downgrades >= 2;
    let queued_slots: HashSet<u8> = building_queue.iter().map(|item| item.slot_id).collect();
    let mut options: Vec<BuildingDowngradeOptionDto> = if can_downgrade && !queue_full {
        village
            .buildings()
            .iter()
            .filter(|building| building.slot_id > 18)
            .filter(|building| building.building.level > 0)
            .filter(|building| !queued_slots.contains(&building.slot_id))
            .map(|building| BuildingDowngradeOptionDto {
                slot_id: building.slot_id,
                building_name: building_key(&building.building.name),
                current_level: building.building.level,
                next_level: building.building.level.saturating_sub(1),
            })
            .collect()
    } else {
        vec![]
    };
    options.sort_by_key(|option| option.slot_id);

    let now = Utc::now();
    let queue = building_queue
        .iter()
        .filter(|item| matches!(item.kind, BuildingWorkflowKind::Downgrade))
        .map(|item| MainBuildingDowngradeQueueItemDto {
            action_id: item.job_id.to_string(),
            slot_id: item.slot_id,
            building_name: building_key(&item.building_name),
            target_level: item.target_level,
            finishes_at: item.finishes_at,
            time_seconds: (item.finishes_at - now).num_seconds().max(0) as u32,
            is_processing: matches!(item.status, ScheduledActionStatus::Processing),
        })
        .collect();

    MainBuildingDetailDto {
        can_downgrade,
        queue_full,
        options,
        queue,
    }
}

fn marketplace_offer_to_dto(
    data: &MarketplaceData,
    offer: &MarketplaceOffer,
    current_position: &Position,
    merchant_speed: u8,
    world_size: i32,
    server_speed: u8,
) -> MarketplaceOfferDto {
    let village_reference = data
        .village_references
        .get(&offer.village_id)
        .expect("Village reference should exist for marketplace offer");
    let distance = current_position.distance(&village_reference.position, world_size);
    let travel_time_seconds = current_position.calculate_travel_time_secs(
        village_reference.position.clone(),
        merchant_speed,
        world_size,
        server_speed.max(1),
    );
    MarketplaceOfferDto {
        offer_id: offer.id.to_string(),
        village_id: offer.village_id,
        village_name: village_reference.name.clone(),
        position: position_to_dto(&village_reference.position),
        distance,
        travel_time_seconds,
        offer_resources: resource_group_to_dto(&offer.offer_resources.into()),
        seek_resources: resource_group_to_dto(&offer.seek_resources.into()),
        merchants_required: offer.merchants_required,
        created_at: offer.created_at.timestamp(),
    }
}

fn merchant_movement_to_dto(
    data: &MarketplaceData,
    movement: &MerchantMovement,
    direction: MerchantMovementDirectionDto,
) -> MerchantMovementDto {
    let origin_info = data.village_references.get(&movement.origin_village_id);
    let destination_info = data
        .village_references
        .get(&movement.destination_village_id);
    MerchantMovementDto {
        job_id: movement.job_id.to_string(),
        direction,
        kind: match movement.kind {
            MerchantMovementKind::Going => MerchantMovementKindDto::Going,
            MerchantMovementKind::Return => MerchantMovementKindDto::Return,
        },
        origin_name: origin_info
            .map(|info| info.name.clone())
            .unwrap_or_else(|| format!("Village #{}", movement.origin_village_id)),
        origin_position: origin_info.map(|info| position_to_dto(&info.position)),
        destination_name: destination_info
            .map(|info| info.name.clone())
            .unwrap_or_else(|| format!("Village #{}", movement.destination_village_id)),
        destination_position: destination_info.map(|info| position_to_dto(&info.position)),
        resources: resource_group_to_dto(&movement.resources),
        merchants_used: movement.merchants_used,
        arrives_at: movement.arrives_at,
    }
}

fn paired_outgoing_going_exists(
    data: &MarketplaceData,
    return_movement: &MerchantMovement,
    outgoing_goings: &[&MerchantMovement],
) -> bool {
    let return_origin = data
        .village_references
        .get(&return_movement.origin_village_id)
        .map(|info| &info.position);
    let return_destination = data
        .village_references
        .get(&return_movement.destination_village_id)
        .map(|info| &info.position);

    outgoing_goings.iter().any(|going| {
        let going_origin = data
            .village_references
            .get(&going.origin_village_id)
            .map(|info| &info.position);
        let going_destination = data
            .village_references
            .get(&going.destination_village_id)
            .map(|info| &info.position);
        let reverse_route_match =
            going_origin == return_destination && going_destination == return_origin;
        let legacy_loopback_return = return_origin == return_destination;
        (reverse_route_match || legacy_loopback_return)
            && going.merchants_used == return_movement.merchants_used
            && going.arrives_at <= return_movement.arrives_at
    })
}

fn position_to_dto(position: &Position) -> PositionDto {
    PositionDto {
        x: position.x,
        y: position.y,
    }
}

fn building_key(name: &BuildingName) -> String {
    format!("{name:?}")
}

fn unit_key(name: &UnitName) -> String {
    format!("{name:?}")
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
            next_upkeep: cost.upkeep,
            upkeep: cost.upkeep,
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
    army_state: Option<&VillageArmyStateView>,
) -> Vec<TrainingUnitOptionDto> {
    if !expected_buildings.contains(&building.building.name) {
        return vec![];
    }

    let training_multiplier = building.building.value as f64 / 1000.0;
    let available_units = village.available_units_for_training(group);
    let tribe = village.tribe.clone();

    let chiefs_at_home = army_state
        .and_then(|state| state.home_army.as_ref())
        .map(|army| army.units().get(8))
        .unwrap_or_else(|| village.count_chiefs_at_home());
    let settlers_at_home = army_state
        .and_then(|state| state.home_army.as_ref())
        .map(|army| army.units().get(9))
        .unwrap_or_else(|| village.count_settlers_at_home());
    let chiefs_deployed: u32 = army_state
        .map(|state| {
            state
                .deployed_armies
                .iter()
                .map(|army| army.units().get(8))
                .sum()
        })
        .unwrap_or_else(|| {
            village
                .deployed_armies()
                .iter()
                .map(|army| army.units().get(8))
                .sum()
        });
    let settlers_deployed: u32 = army_state
        .map(|state| {
            state
                .deployed_armies
                .iter()
                .map(|army| army.units().get(9))
                .sum()
        })
        .unwrap_or_else(|| {
            village
                .deployed_armies()
                .iter()
                .map(|army| army.units().get(9))
                .sum()
        });
    let mut chiefs_queued = 0u32;
    let mut settlers_queued = 0u32;
    for item in training_queue {
        let qty = item.quantity.max(0) as u32;
        if matches!(
            item.unit,
            UnitName::Chief | UnitName::Senator | UnitName::Chieftain
        ) {
            chiefs_queued = chiefs_queued.saturating_add(qty);
        } else if matches!(item.unit, UnitName::Settler) {
            settlers_queued = settlers_queued.saturating_add(qty);
        }
    }
    let chiefs_total = chiefs_at_home + chiefs_deployed + chiefs_queued + moving_expansion_units.0;
    let settlers_total =
        settlers_at_home + settlers_deployed + settlers_queued + moving_expansion_units.1;
    let available_slots = foundation_free_slots.unwrap_or_else(|| village.max_foundation_slots());
    let committed_expansion_slots =
        parabellum_game::models::village::Village::slots_used_by_chiefs(chiefs_total)
            + parabellum_game::models::village::Village::slots_used_by_settlers(settlers_total);
    let free_expansion_slots = available_slots.saturating_sub(committed_expansion_slots as u8);

    available_units
        .into_iter()
        .filter_map(|unit| {
            let max_trainable = if matches!(unit.role, UnitRole::Chief | UnitRole::Settler) {
                let max_trainable = match unit.role {
                    UnitRole::Chief => {
                        parabellum_game::models::village::Village::max_chiefs_for_slots(
                            free_expansion_slots,
                        )
                    }
                    UnitRole::Settler => {
                        parabellum_game::models::village::Village::max_settlers_for_slots(
                            free_expansion_slots,
                        )
                    }
                    _ => 0,
                };
                if max_trainable == 0 {
                    return None;
                }
                Some(max_trainable)
            } else {
                None
            };
            let unit_idx = tribe.get_unit_idx_by_name(&unit.name)? as u8;
            let base_time_per_unit = unit.cost.time as f64 / server_speed as f64;
            let trough_multiplier = village.cavalry_training_time_multiplier(unit);
            let time_per_unit = (base_time_per_unit * training_multiplier * trough_multiplier)
                .floor()
                .max(1.0) as u32;

            Some(TrainingUnitOptionDto {
                unit_idx,
                name: unit_key(&unit.name),
                max_trainable,
                cost: resource_group_to_dto(&unit.cost.resources),
                upkeep: village.effective_unit_upkeep(unit),
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
        .map(|(idx, unit)| {
            unit.cost
                .upkeep
                .saturating_mul(*units.get(idx).unwrap_or(&0))
        })
        .sum()
}

fn troop_upkeep_for_rally_card(
    village: &parabellum_game::models::village::Village,
    card: &ArmyCardData,
) -> Option<u32> {
    let is_returning_own_movement = matches!(
        (card.category, card.movement_kind),
        (ArmyCategory::Incoming, Some(MovementKind::Return))
    );
    let is_hidden_incoming_movement =
        matches!(card.category, ArmyCategory::Incoming) && !is_returning_own_movement;
    let is_stationed_away_from_current_village = matches!(card.category, ArmyCategory::Deployed);
    if is_hidden_incoming_movement || is_stationed_away_from_current_village {
        return None;
    }

    let is_own_context = matches!(
        card.category,
        ArmyCategory::Stationed | ArmyCategory::Outgoing
    ) || is_returning_own_movement;
    if is_own_context && card.tribe == village.tribe {
        Some(
            card.tribe
                .units()
                .iter()
                .enumerate()
                .map(|(idx, unit)| {
                    village
                        .effective_unit_upkeep(unit)
                        .saturating_mul(*card.units.units().get(idx).unwrap_or(&0))
                })
                .sum(),
        )
    } else {
        Some(troop_upkeep_for_tribe(&card.tribe, card.units.units()))
    }
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
        let next_level = effective_level.saturating_add(1).min(smithy_level_cap);
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
            next_level,
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
