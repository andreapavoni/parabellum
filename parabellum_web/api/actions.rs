//! Mutation handlers for game actions.
//!
//! This module contains command-style endpoints (build, train, send troops/resources,
//! marketplace actions, research, reinforce/recall, and village founding).
//! Handlers stay orchestration-only: validate request shape, resolve authenticated user,
//! invoke `parabellum_app` command handlers, and map errors to API codes.

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_app::{
    command_handlers::{
        AcceptMarketplaceOfferCommandHandler, AddBuildingCommandHandler,
        AttackVillageCommandHandler, CancelMarketplaceOfferCommandHandler,
        CreateMarketplaceOfferCommandHandler, FoundVillageCommandHandler,
        RecallTroopsCommandHandler, ReinforceVillageCommandHandler,
        ReleaseReinforcementsCommandHandler, ResearchAcademyCommandHandler,
        ResearchSmithyCommandHandler, ScoutVillageCommandHandler, SendResourcesCommandHandler,
        TrainUnitsCommandHandler, UpgradeBuildingCommandHandler,
    },
    cqrs::commands::{
        AcceptMarketplaceOffer, AddBuilding, AttackVillage, CancelMarketplaceOffer,
        CreateMarketplaceOffer, FoundVillage, RecallTroops, ReinforceVillage,
        ReleaseReinforcements, ResearchAcademy, ResearchSmithy, ScoutVillage, SendResources,
        TrainUnits, UpgradeBuilding,
    },
};
use parabellum_types::{
    army::{TroopSet, UnitName},
    battle::{AttackType, ScoutingTarget},
    buildings::BuildingName,
    common::ResourceGroup,
    map::Position,
};

use crate::api::errors::ApiError;
use crate::http::AppState;

use super::authenticated_user;

const MAX_SLOT_ID: u8 = 40;
const RALLY_POINT_SLOT: u8 = 39;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Generic success response for command endpoints.
pub struct ActionResponse {
    pub success: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Payload for building creation on an empty slot.
pub struct AddBuildingRequest {
    pub slot_id: u8,
    pub building_name: BuildingName,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Payload for upgrading a building by slot.
pub struct UpgradeBuildingRequest {
    pub slot_id: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Payload for unit training command.
pub struct TrainUnitsRequest {
    pub slot_id: u8,
    pub unit_idx: u8,
    pub quantity: i32,
    pub building_name: BuildingName,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Payload for academy research.
pub struct ResearchAcademyRequest {
    pub slot_id: u8,
    pub unit_name: UnitName,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Payload for smithy upgrade research.
pub struct ResearchSmithyRequest {
    pub slot_id: u8,
    pub unit_name: UnitName,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Payload for direct resource transfer via marketplace.
pub struct SendResourcesRequest {
    pub slot_id: u8,
    pub target_x: i32,
    pub target_y: i32,
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Payload for marketplace offer creation.
pub struct CreateOfferRequest {
    pub slot_id: u8,
    pub offer_lumber: u32,
    pub offer_clay: u32,
    pub offer_iron: u32,
    pub offer_crop: u32,
    pub seek_lumber: u32,
    pub seek_clay: u32,
    pub seek_iron: u32,
    pub seek_crop: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Common payload for marketplace offer accept/cancel actions.
pub struct OfferActionRequest {
    pub slot_id: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MovementKind {
    Attack,
    Raid,
    Reinforcement,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScoutingTargetKind {
    Resources,
    Defenses,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Payload for troop movement commands.
pub struct SendTroopsRequest {
    pub slot_id: u8,
    pub target_x: i32,
    pub target_y: i32,
    pub movement: MovementKind,
    pub units: Vec<i32>,
    pub scouting_target: Option<ScoutingTargetKind>,
    pub catapult_targets: Option<Vec<BuildingName>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Payload for recalling deployed units.
pub struct RecallTroopsRequest {
    pub army_id: Uuid,
    pub units: Vec<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Payload for releasing reinforcements from a source village.
pub struct ReleaseReinforcementsRequest {
    pub source_village_id: u32,
    pub units: Vec<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Payload for settler village founding movement.
pub struct FoundVillageRequest {
    pub target_x: i32,
    pub target_y: i32,
    pub units: Vec<i32>,
}

/// Starts a building construction job on an empty slot.
pub async fn add_building(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AddBuildingRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;

    state
        .app_bus
        .execute(
            AddBuilding {
                player_id: user.player.id,
                village_id: user.village.id,
                slot_id: payload.slot_id,
                name: payload.building_name,
            },
            AddBuildingCommandHandler::new(),
        )
        .await
        .map_err(|err| ApiError::unprocessable(err.to_string()))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Queues a building upgrade for the target slot.
pub async fn upgrade_building(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpgradeBuildingRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;

    state
        .app_bus
        .execute(
            UpgradeBuilding {
                player_id: user.player.id,
                village_id: user.village.id,
                slot_id: payload.slot_id,
            },
            UpgradeBuildingCommandHandler::new(),
        )
        .await
        .map_err(|err| ApiError::unprocessable(err.to_string()))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Queues unit training in a valid training/expansion building slot.
pub async fn train_units(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TrainUnitsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;

    if payload.quantity <= 0 {
        return Err(ApiError::unprocessable(
            "Training quantity must be greater than zero",
        ));
    }

    ensure_building_in_slot(
        &user.village,
        payload.slot_id,
        payload.building_name.clone(),
    )?;

    state
        .app_bus
        .execute(
            TrainUnits {
                player_id: user.player.id,
                village_id: user.village.id,
                unit_idx: payload.unit_idx,
                quantity: payload.quantity,
                building_name: payload.building_name,
            },
            TrainUnitsCommandHandler::new(),
        )
        .await
        .map_err(|err| ApiError::unprocessable(err.to_string()))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Queues academy research for a unit.
pub async fn research_academy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ResearchAcademyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;
    ensure_building_in_slot(&user.village, payload.slot_id, BuildingName::Academy)?;

    state
        .app_bus
        .execute(
            ResearchAcademy {
                unit: payload.unit_name,
                village_id: user.village.id,
            },
            ResearchAcademyCommandHandler::new(),
        )
        .await
        .map_err(|err| ApiError::unprocessable(err.to_string()))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Queues smithy research for a unit.
pub async fn research_smithy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ResearchSmithyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;
    ensure_building_in_slot(&user.village, payload.slot_id, BuildingName::Smithy)?;

    state
        .app_bus
        .execute(
            ResearchSmithy {
                unit: payload.unit_name,
                village_id: user.village.id,
            },
            ResearchSmithyCommandHandler::new(),
        )
        .await
        .map_err(|err| ApiError::unprocessable(err.to_string()))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Sends resources from current village to coordinates-derived target village.
pub async fn send_resources(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SendResourcesRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;
    ensure_building_in_slot(&user.village, payload.slot_id, BuildingName::Marketplace)?;

    let target_position = Position {
        x: payload.target_x,
        y: payload.target_y,
    };
    let target_village_id = target_position.to_id(state.world_size);

    state
        .app_bus
        .execute(
            SendResources {
                player_id: user.player.id,
                village_id: user.village.id,
                target_village_id,
                resources: ResourceGroup(payload.lumber, payload.clay, payload.iron, payload.crop),
            },
            SendResourcesCommandHandler::new(),
        )
        .await
        .map_err(|err| ApiError::unprocessable(err.to_string()))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Creates a marketplace offer from current village.
pub async fn create_marketplace_offer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateOfferRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;
    ensure_building_in_slot(&user.village, payload.slot_id, BuildingName::Marketplace)?;

    state
        .app_bus
        .execute(
            CreateMarketplaceOffer {
                village_id: user.village.id,
                offer_resources: ResourceGroup(
                    payload.offer_lumber,
                    payload.offer_clay,
                    payload.offer_iron,
                    payload.offer_crop,
                ),
                seek_resources: ResourceGroup(
                    payload.seek_lumber,
                    payload.seek_clay,
                    payload.seek_iron,
                    payload.seek_crop,
                ),
            },
            CreateMarketplaceOfferCommandHandler::new(),
        )
        .await
        .map_err(|err| ApiError::unprocessable(err.to_string()))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Accepts an existing marketplace offer.
pub async fn accept_marketplace_offer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(offer_id): Path<Uuid>,
    Json(payload): Json<OfferActionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;
    ensure_building_in_slot(&user.village, payload.slot_id, BuildingName::Marketplace)?;

    state
        .app_bus
        .execute(
            AcceptMarketplaceOffer {
                player_id: user.player.id,
                village_id: user.village.id,
                offer_id,
            },
            AcceptMarketplaceOfferCommandHandler::new(),
        )
        .await
        .map_err(|err| ApiError::unprocessable(err.to_string()))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Cancels one of current village marketplace offers.
pub async fn cancel_marketplace_offer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(offer_id): Path<Uuid>,
    Json(payload): Json<OfferActionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;
    ensure_building_in_slot(&user.village, payload.slot_id, BuildingName::Marketplace)?;

    state
        .app_bus
        .execute(
            CancelMarketplaceOffer {
                player_id: user.player.id,
                village_id: user.village.id,
                offer_id,
            },
            CancelMarketplaceOfferCommandHandler::new(),
        )
        .await
        .map_err(|err| ApiError::unprocessable(err.to_string()))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Sends troops as attack/raid/reinforcement or scouting movement.
pub async fn send_troops(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SendTroopsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;
    ensure_building_in_slot(&user.village, payload.slot_id, BuildingName::RallyPoint)?;
    ensure_rally_point_slot(payload.slot_id)?;

    let home_army = user
        .village
        .army()
        .ok_or_else(|| ApiError::unprocessable("No village army available"))?;
    let units = parse_troop_set(&payload.units)?;

    if units.units().iter().all(|value| *value == 0) {
        return Err(ApiError::unprocessable("At least one unit is required"));
    }

    let position = Position {
        x: payload.target_x,
        y: payload.target_y,
    };
    let target_village_id = position.to_id(state.world_size);

    if let Some(target) = payload.scouting_target {
        let attack_type = match payload.movement {
            MovementKind::Attack => AttackType::Normal,
            MovementKind::Raid => AttackType::Raid,
            MovementKind::Reinforcement => {
                return Err(ApiError::bad_request(
                    "Scouting is only available for attack or raid movements",
                ));
            }
        };

        let scouting_target = match target {
            ScoutingTargetKind::Resources => ScoutingTarget::Resources,
            ScoutingTargetKind::Defenses => ScoutingTarget::Defenses,
        };

        state
            .app_bus
            .execute(
                ScoutVillage {
                    player_id: user.player.id,
                    village_id: user.village.id,
                    army_id: home_army.id,
                    units,
                    target_village_id,
                    target: scouting_target,
                    attack_type,
                },
                ScoutVillageCommandHandler::new(),
            )
            .await
            .map_err(|err| ApiError::unprocessable(err.to_string()))?;

        return Ok(Json(ActionResponse { success: true }));
    }

    match payload.movement {
        MovementKind::Attack | MovementKind::Raid => {
            let catapult_targets = parse_catapult_targets(payload.catapult_targets)?;

            state
                .app_bus
                .execute(
                    AttackVillage {
                        player_id: user.player.id,
                        village_id: user.village.id,
                        army_id: home_army.id,
                        units,
                        target_village_id,
                        catapult_targets,
                        hero_id: None,
                        attack_type: match payload.movement {
                            MovementKind::Attack => AttackType::Normal,
                            MovementKind::Raid => AttackType::Raid,
                            MovementKind::Reinforcement => AttackType::Normal,
                        },
                    },
                    AttackVillageCommandHandler::new(),
                )
                .await
                .map_err(|err| ApiError::unprocessable(err.to_string()))?;
        }
        MovementKind::Reinforcement => {
            state
                .app_bus
                .execute(
                    ReinforceVillage {
                        player_id: user.player.id,
                        village_id: user.village.id,
                        army_id: home_army.id,
                        units,
                        target_village_id,
                        hero_id: None,
                    },
                    ReinforceVillageCommandHandler::new(),
                )
                .await
                .map_err(|err| ApiError::unprocessable(err.to_string()))?;
        }
    }

    Ok(Json(ActionResponse { success: true }))
}

/// Recalls units from a deployed army.
pub async fn recall_troops(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RecallTroopsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let units = parse_troop_set(&payload.units)?;

    state
        .app_bus
        .execute(
            RecallTroops {
                player_id: user.player.id,
                village_id: user.village.id,
                army_id: payload.army_id,
                units,
            },
            RecallTroopsCommandHandler::new(),
        )
        .await
        .map_err(|err| ApiError::unprocessable(err.to_string()))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Releases reinforcements back to their origin village.
pub async fn release_reinforcements(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ReleaseReinforcementsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let units = parse_troop_set(&payload.units)?;

    state
        .app_bus
        .execute(
            ReleaseReinforcements {
                player_id: user.player.id,
                village_id: user.village.id,
                source_village_id: payload.source_village_id,
                units,
            },
            ReleaseReinforcementsCommandHandler::new(),
        )
        .await
        .map_err(|err| ApiError::unprocessable(err.to_string()))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Sends settlers to found a new village.
pub async fn found_village(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<FoundVillageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    let home_army = user
        .village
        .army()
        .ok_or_else(|| ApiError::unprocessable("No village army available"))?;
    let units = parse_troop_set(&payload.units)?;

    state
        .app_bus
        .execute(
            FoundVillage {
                player_id: user.player.id,
                village_id: user.village.id,
                army_id: home_army.id,
                units,
                target_position: Position {
                    x: payload.target_x,
                    y: payload.target_y,
                },
            },
            FoundVillageCommandHandler::new(),
        )
        .await
        .map_err(|err| ApiError::unprocessable(err.to_string()))?;

    Ok(Json(ActionResponse { success: true }))
}

fn ensure_slot(slot_id: u8) -> Result<(), ApiError> {
    if !(1..=MAX_SLOT_ID).contains(&slot_id) {
        Err(ApiError::bad_request("Invalid building slot"))
    } else {
        Ok(())
    }
}

fn ensure_rally_point_slot(slot_id: u8) -> Result<(), ApiError> {
    if slot_id != RALLY_POINT_SLOT {
        Err(ApiError::unprocessable(
            "Troops can only be sent from the rally point slot",
        ))
    } else {
        Ok(())
    }
}

fn ensure_building_in_slot(
    village: &parabellum_game::models::village::Village,
    slot_id: u8,
    building_name: BuildingName,
) -> Result<(), ApiError> {
    let slot = village
        .get_building_by_slot_id(slot_id)
        .ok_or_else(|| ApiError::bad_request("No building in the selected slot"))?;

    if slot.building.name != building_name {
        return Err(ApiError::unprocessable(
            "Selected building does not match slot",
        ));
    }

    Ok(())
}

fn parse_troop_set(values: &[i32]) -> Result<TroopSet, ApiError> {
    let mut troops = TroopSet::default();
    for idx in 0..troops.units().len() {
        let amount = *values.get(idx).unwrap_or(&0);
        if amount < 0 {
            return Err(ApiError::unprocessable("Unit amounts cannot be negative"));
        }
        troops.set(idx, amount as u32);
    }
    Ok(troops)
}

fn parse_catapult_targets(
    targets: Option<Vec<BuildingName>>,
) -> Result<[BuildingName; 2], ApiError> {
    match targets {
        None => Ok([BuildingName::MainBuilding, BuildingName::Warehouse]),
        Some(values) if values.len() == 2 => Ok([values[0].clone(), values[1].clone()]),
        Some(_) => Err(ApiError::unprocessable(
            "catapultTargets must contain exactly 2 building names",
        )),
    }
}
