//! Mutation handlers for game actions.
//!
//! This module contains command-style endpoints (build, train, send troops/resources,
//! marketplace actions, research, reinforce/recall, and village founding).
//! Handlers stay orchestration-only: validate request shape, resolve authenticated user,
//! invoke `GameApplication`, and map errors to API codes.

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use parabellum_app::ports::villages::{
    AcceptMarketplaceOfferRequest, AddBuildingRequest as AddBuildingUseCaseRequest,
    AssignHeroPointsRequest as AssignHeroPointsUseCaseRequest,
    BuildTrapsRequest as BuildTrapsUseCaseRequest,
    CancelBuildingConstructionRequest as CancelBuildingConstructionUseCaseRequest,
    CancelMarketplaceOfferRequest, CancelTroopMovementRequest as CancelTroopMovementUseCaseRequest,
    CreateMarketplaceOfferRequest,
    DisbandTrappedTroopsRequest as DisbandTrappedTroopsUseCaseRequest,
    DowngradeBuildingRequest as DowngradeBuildingUseCaseRequest,
    RecallReinforcementsRequest as RecallReinforcementsUseCaseRequest,
    ReleaseReinforcementsRequest as ReleaseReinforcementsUseCaseRequest,
    ReleaseTrappedTroopsRequest as ReleaseTrappedTroopsUseCaseRequest,
    RenameVillageRequest as RenameVillageUseCaseRequest,
    ResearchAcademyRequest as ResearchAcademyUseCaseRequest,
    ResearchSmithyRequest as ResearchSmithyUseCaseRequest,
    ResetHeroPointsRequest as ResetHeroPointsUseCaseRequest,
    ReviveHeroRequest as ReviveHeroUseCaseRequest, SendAttackRequest as SendAttackUseCaseRequest,
    SendReinforcementRequest as SendReinforcementUseCaseRequest,
    SendResourcesRequest as SendResourcesUseCaseRequest,
    SendScoutRequest as SendScoutUseCaseRequest, SendSettlersRequest as SendSettlersUseCaseRequest,
    SetHeroResourceFocusRequest as SetHeroResourceFocusUseCaseRequest,
    TrainUnitsRequest as TrainUnitsUseCaseRequest,
    UpgradeBuildingRequest as UpgradeBuildingUseCaseRequest,
};
use parabellum_game::models::hero::HeroResourceFocus;
use parabellum_types::{
    army::{TroopSet, UnitName},
    battle::{AttackType, ScoutingTarget},
    buildings::BuildingName,
    common::{ResourceGroup, ResourceKind, ResourceQuantity},
    map::Position,
};

use crate::api::dto::ResourceAmountsDto;
use crate::api::errors::ApiError;
use crate::http::AppState;

use super::authenticated_user;
use super::error_mapping::map_application_error;

const MAX_SLOT_ID: u8 = 40;
const RALLY_POINT_SLOT: u8 = 39;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Generic success response for command endpoints.
pub struct ActionResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Current player hero state.
pub struct HeroResponse {
    pub id: Uuid,
    pub village_id: u32,
    #[schema(value_type = String)]
    pub tribe: parabellum_types::tribe::Tribe,
    pub level: u16,
    pub health: u16,
    pub experience: u32,
    pub xp_for_next_level: u32,
    #[schema(value_type = String)]
    pub resource_focus: HeroResourceFocus,
    pub strength_points: u16,
    pub off_bonus_points: u16,
    pub def_bonus_points: u16,
    pub regeneration_points: u16,
    pub resources_points: u16,
    pub unassigned_points: u16,
    pub speed: u8,
    pub strength_value: u32,
    pub strength_per_point: u32,
    pub off_bonus_percent: f64,
    pub off_bonus_percent_per_point: f64,
    pub def_bonus_percent: f64,
    pub def_bonus_percent_per_point: f64,
    pub regeneration_percent_per_day: u16,
    pub regeneration_percent_per_point: u16,
    pub resource_production: ResourceAmountsDto,
    pub resurrection_cost: ResourceAmountsDto,
    pub resurrection_time_secs: u32,
    #[schema(value_type = Option<String>)]
    pub revival_finishes_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for building creation on an empty slot.
pub struct AddBuildingRequest {
    pub slot_id: u8,
    #[schema(value_type = String)]
    pub building_name: BuildingName,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for upgrading a building by slot.
pub struct UpgradeBuildingRequest {
    pub slot_id: u8,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for downgrading a building by slot.
pub struct DowngradeBuildingRequest {
    pub slot_id: u8,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for canceling a queued building action.
pub struct CancelBuildingConstructionRequest {
    pub action_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for village rename action.
pub struct RenameVillageRequest {
    pub village_id: u32,
    pub village_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for unit training command.
pub struct TrainUnitsRequest {
    pub slot_id: u8,
    pub unit_idx: u8,
    pub quantity: i32,
    #[schema(value_type = String)]
    pub building_name: BuildingName,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for assigning hero points.
pub struct AssignHeroPointsRequest {
    pub hero_id: Uuid,
    pub village_id: u32,
    pub strength: u16,
    pub off_bonus: u16,
    pub def_bonus: u16,
    pub regeneration: u16,
    pub resources: u16,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for resetting a level-0 hero's assigned points.
pub struct ResetHeroPointsRequest {
    pub hero_id: Uuid,
    pub village_id: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for changing hero resource focus.
pub struct SetHeroResourceFocusRequest {
    pub hero_id: Uuid,
    pub village_id: u32,
    #[schema(value_type = String)]
    pub focus: HeroResourceFocus,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for reviving a dead hero.
pub struct ReviveHeroRequest {
    pub hero_id: Uuid,
    pub village_id: u32,
    pub reset: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for academy research.
pub struct ResearchAcademyRequest {
    pub slot_id: u8,
    #[schema(value_type = String)]
    pub unit_name: UnitName,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for smithy upgrade research.
pub struct ResearchSmithyRequest {
    pub slot_id: u8,
    #[schema(value_type = String)]
    pub unit_name: UnitName,
}

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Common payload for marketplace offer accept/cancel actions.
pub struct OfferActionRequest {
    pub slot_id: u8,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MovementKind {
    Attack,
    Raid,
    Reinforcement,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScoutingTargetKind {
    Resources,
    Defenses,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for troop movement commands.
pub struct SendTroopsRequest {
    pub slot_id: u8,
    pub target_x: i32,
    pub target_y: i32,
    pub movement: MovementKind,
    pub units: Vec<i32>,
    pub hero_id: Option<Uuid>,
    pub scouting_target: Option<ScoutingTargetKind>,
    #[schema(value_type = Option<Vec<String>>)]
    pub catapult_targets: Option<Vec<CatapultTargetInput>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CatapultTargetInput {
    Building(BuildingName),
    Text(String),
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for recalling deployed units.
pub struct RecallTroopsRequest {
    pub village_id: u32,
    pub army_id: Uuid,
    pub units: Vec<i32>,
    pub hero_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for releasing reinforcements from a source village.
pub struct ReleaseReinforcementsRequest {
    pub village_id: u32,
    pub army_id: Uuid,
    pub units: Vec<i32>,
    pub hero_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseTrappedTroopsRequest {
    pub village_id: u32,
    pub army_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DisbandTrappedTroopsRequest {
    pub village_id: u32,
    pub army_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildTrapsRequest {
    pub village_id: u32,
    pub quantity: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Payload for settler village founding movement.
pub struct FoundVillageRequest {
    pub target_x: i32,
    pub target_y: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewTroopsRequest {
    pub target_x: i32,
    pub target_y: i32,
    pub movement: MovementKind,
    pub units: Vec<i32>,
    pub hero_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSendResourcesRequest {
    pub slot_id: u8,
    pub target_x: i32,
    pub target_y: i32,
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewFoundVillageRequest {
    pub target_x: i32,
    pub target_y: i32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MovementPreviewResponse {
    #[schema(value_type = String)]
    pub arrives_at: chrono::DateTime<chrono::Utc>,
    pub distance: u32,
    pub detected_kind: PreviewDetectedKind,
    pub supports_scouting_target_choice: bool,
    pub has_catapult_units: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PreviewDetectedKind {
    AttackOrRaid,
    ScoutOnly,
    Reinforcement,
    FoundVillage,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendResourcesPreviewResponse {
    #[schema(value_type = String)]
    pub arrives_at: chrono::DateTime<chrono::Utc>,
}

/// Starts a building construction job on an empty slot.
#[utoipa::path(
    post,
    path = "/buildings/add",
    request_body = AddBuildingRequest,
    responses((status = 200, body = ActionResponse))
)]
pub async fn add_building(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AddBuildingRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;

    state
        .game_app
        .add_building(AddBuildingUseCaseRequest {
            player_id: user.player.id,
            village_id: user.village.id,
            slot_id: payload.slot_id,
            building_name: payload.building_name,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Queues a building upgrade for the target slot.
#[utoipa::path(
    post,
    path = "/buildings/upgrade",
    request_body = UpgradeBuildingRequest,
    responses((status = 200, body = ActionResponse))
)]
pub async fn upgrade_building(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpgradeBuildingRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;

    state
        .game_app
        .upgrade_building(UpgradeBuildingUseCaseRequest {
            player_id: user.player.id,
            village_id: user.village.id,
            slot_id: payload.slot_id,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Queues a Main Building downgrade for the target slot.
#[utoipa::path(
    post,
    path = "/buildings/downgrade",
    request_body = DowngradeBuildingRequest,
    responses((status = 200, body = ActionResponse))
)]
pub async fn downgrade_building(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DowngradeBuildingRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;

    state
        .game_app
        .downgrade_building(DowngradeBuildingUseCaseRequest {
            player_id: user.player.id,
            village_id: user.village.id,
            slot_id: payload.slot_id,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Cancels a queued building add, upgrade, or downgrade action.
#[utoipa::path(
    post,
    path = "/buildings/cancel",
    request_body = CancelBuildingConstructionRequest,
    responses((status = 200, body = ActionResponse))
)]
pub async fn cancel_building_construction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CancelBuildingConstructionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    state
        .game_app
        .cancel_building_construction(CancelBuildingConstructionUseCaseRequest {
            player_id: user.player.id,
            village_id: user.village.id,
            action_id: payload.action_id,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Renames an owned village.
#[utoipa::path(
    post,
    path = "/villages/rename",
    request_body = RenameVillageRequest,
    responses((status = 200, body = ActionResponse))
)]
pub async fn rename_village(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RenameVillageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    state
        .game_app
        .rename_village(RenameVillageUseCaseRequest {
            player_id: user.player.id,
            village_id: payload.village_id,
            village_name: payload.village_name,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Queues unit training in a valid training/expansion building slot.
#[utoipa::path(
    post,
    path = "/army/train",
    request_body = TrainUnitsRequest,
    responses((status = 200, body = ActionResponse))
)]
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
        .game_app
        .train_units(TrainUnitsUseCaseRequest {
            player_id: user.player.id,
            village_id: user.village.id,
            unit_idx: payload.unit_idx,
            quantity: payload.quantity,
            building_name: payload.building_name,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Queues academy research for a unit.
#[utoipa::path(
    post,
    path = "/academy/research",
    request_body = ResearchAcademyRequest,
    responses((status = 200, body = ActionResponse))
)]
pub async fn research_academy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ResearchAcademyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;
    ensure_building_in_slot(&user.village, payload.slot_id, BuildingName::Academy)?;

    state
        .game_app
        .research_academy(ResearchAcademyUseCaseRequest {
            player_id: user.player.id,
            village_id: user.village.id,
            unit: payload.unit_name,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Queues smithy research for a unit.
#[utoipa::path(
    post,
    path = "/smithy/research",
    request_body = ResearchSmithyRequest,
    responses((status = 200, body = ActionResponse))
)]
pub async fn research_smithy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ResearchSmithyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;
    ensure_building_in_slot(&user.village, payload.slot_id, BuildingName::Smithy)?;

    state
        .game_app
        .research_smithy(ResearchSmithyUseCaseRequest {
            player_id: user.player.id,
            village_id: user.village.id,
            unit: payload.unit_name,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Sends resources from current village to coordinates-derived target village.
#[utoipa::path(
    post,
    path = "/marketplace/send",
    request_body = SendResourcesRequest,
    responses((status = 200, body = ActionResponse))
)]
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
        .game_app
        .send_resources(SendResourcesUseCaseRequest {
            player_id: user.player.id,
            source_village_id: user.village.id,
            target_village_id,
            resources: ResourceGroup(payload.lumber, payload.clay, payload.iron, payload.crop),
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

pub async fn current_hero(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let hero = state
        .game_app
        .get_hero_by_player(user.player.id)
        .await
        .map_err(|err| map_application_error("unable_to_load_hero", err))?
        .ok_or_else(|| ApiError::not_found("Hero not found"))?;
    let strength_per_point = hero
        .tribe
        .get_top_unit()
        .map(|unit| {
            unit.attack
                .max(unit.defense_infantry)
                .max(unit.defense_cavalry)
        })
        .unwrap_or(0);
    let resource_production = hero.resources();
    let resurrection_cost = hero.resurrection_cost(state.server_speed);
    let revival_finishes_at = state
        .game_app
        .get_pending_hero_revival(user.player.id)
        .await
        .map_err(|err| map_application_error("unable_to_load_hero_revival", err))?;

    Ok(Json(HeroResponse {
        id: hero.id,
        village_id: hero.village_id,
        tribe: hero.tribe.clone(),
        level: hero.level,
        health: hero.health,
        experience: hero.experience,
        xp_for_next_level: hero.xp_for_next_level(),
        resource_focus: hero.resource_focus,
        strength_points: hero.strength_points,
        off_bonus_points: hero.off_bonus_points,
        def_bonus_points: hero.def_bonus_points,
        regeneration_points: hero.regeneration_points,
        resources_points: hero.resources_points,
        unassigned_points: hero.unassigned_points,
        speed: hero.speed(),
        strength_value: hero.strength(),
        strength_per_point,
        off_bonus_percent: (hero.off_bonus() - 1.0) * 100.0,
        off_bonus_percent_per_point: 0.2,
        def_bonus_percent: (hero.def_bonus() - 1.0) * 100.0,
        def_bonus_percent_per_point: 0.2,
        regeneration_percent_per_day: hero.regeneration(),
        regeneration_percent_per_point: 5,
        resource_production: ResourceAmountsDto {
            lumber: resource_production.lumber(),
            clay: resource_production.clay(),
            iron: resource_production.iron(),
            crop: resource_production.crop(),
        },
        resurrection_cost: ResourceAmountsDto {
            lumber: resurrection_cost.resources.lumber(),
            clay: resurrection_cost.resources.clay(),
            iron: resurrection_cost.resources.iron(),
            crop: resurrection_cost.resources.crop(),
        },
        resurrection_time_secs: resurrection_cost.time,
        revival_finishes_at,
    }))
}

pub async fn assign_hero_points(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AssignHeroPointsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    state
        .game_app
        .assign_hero_points(AssignHeroPointsUseCaseRequest {
            hero_id: payload.hero_id,
            player_id: user.player.id,
            village_id: payload.village_id,
            strength: payload.strength,
            off_bonus: payload.off_bonus,
            def_bonus: payload.def_bonus,
            regeneration: payload.regeneration,
            resources: payload.resources,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

pub async fn reset_hero_points(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ResetHeroPointsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    state
        .game_app
        .reset_hero_points(ResetHeroPointsUseCaseRequest {
            hero_id: payload.hero_id,
            player_id: user.player.id,
            village_id: payload.village_id,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

pub async fn set_hero_resource_focus(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SetHeroResourceFocusRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    state
        .game_app
        .set_hero_resource_focus(SetHeroResourceFocusUseCaseRequest {
            hero_id: payload.hero_id,
            player_id: user.player.id,
            village_id: payload.village_id,
            focus: payload.focus,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

pub async fn revive_hero(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ReviveHeroRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    state
        .game_app
        .revive_hero(ReviveHeroUseCaseRequest {
            hero_id: payload.hero_id,
            player_id: user.player.id,
            village_id: payload.village_id,
            reset: payload.reset,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

#[utoipa::path(
    post,
    path = "/marketplace/send/preview",
    request_body = PreviewSendResourcesRequest,
    responses((status = 200, body = SendResourcesPreviewResponse))
)]
pub async fn preview_send_resources(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PreviewSendResourcesRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;
    ensure_building_in_slot(&user.village, payload.slot_id, BuildingName::Marketplace)?;

    let total_resources = payload.lumber + payload.clay + payload.iron + payload.crop;
    if total_resources == 0 {
        return Err(ApiError::unprocessable(
            "At least one resource amount must be greater than zero",
        ));
    }

    let target_position = Position {
        x: payload.target_x,
        y: payload.target_y,
    };
    let merchant_speed = user.village.tribe.merchant_stats().speed;
    let travel_time_secs = user.village.position.calculate_travel_time_secs(
        target_position,
        merchant_speed,
        state.world_size,
        state.server_speed as u8,
    );
    let arrives_at =
        chrono::Utc::now() + chrono::Duration::seconds(std::cmp::max(1, travel_time_secs) as i64);

    Ok(Json(SendResourcesPreviewResponse { arrives_at }))
}

/// Creates a marketplace offer from current village.
#[utoipa::path(
    post,
    path = "/marketplace/offers",
    request_body = CreateOfferRequest,
    responses((status = 200, body = ActionResponse))
)]
pub async fn create_marketplace_offer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateOfferRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;
    ensure_building_in_slot(&user.village, payload.slot_id, BuildingName::Marketplace)?;

    let offer_resources = parse_resource_quantity(
        payload.offer_lumber,
        payload.offer_clay,
        payload.offer_iron,
        payload.offer_crop,
    )?;
    let seek_resources = parse_resource_quantity(
        payload.seek_lumber,
        payload.seek_clay,
        payload.seek_iron,
        payload.seek_crop,
    )?;

    state
        .game_app
        .create_marketplace_offer(CreateMarketplaceOfferRequest {
            player_id: user.player.id,
            village_id: user.village.id,
            offer_resources,
            seek_resources,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Accepts an existing marketplace offer.
#[utoipa::path(
    post,
    path = "/marketplace/offers/{offer_id}/accept",
    params(
        ("offer_id" = Uuid, Path, description = "Marketplace offer id")
    ),
    request_body = OfferActionRequest,
    responses((status = 200, body = ActionResponse))
)]
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
        .game_app
        .accept_marketplace_offer(AcceptMarketplaceOfferRequest {
            player_id: user.player.id,
            village_id: user.village.id,
            offer_id,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Cancels one of current village marketplace offers.
#[utoipa::path(
    post,
    path = "/marketplace/offers/{offer_id}/cancel",
    params(
        ("offer_id" = Uuid, Path, description = "Marketplace offer id")
    ),
    request_body = OfferActionRequest,
    responses((status = 200, body = ActionResponse))
)]
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
        .game_app
        .cancel_marketplace_offer(CancelMarketplaceOfferRequest {
            player_id: user.player.id,
            village_id: user.village.id,
            offer_id,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Sends troops as attack/raid/reinforcement or scouting movement.
#[utoipa::path(
    post,
    path = "/army/send",
    request_body = SendTroopsRequest,
    responses((status = 200, body = ActionResponse))
)]
pub async fn send_troops(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SendTroopsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    ensure_slot(payload.slot_id)?;
    ensure_building_in_slot(&user.village, payload.slot_id, BuildingName::RallyPoint)?;
    ensure_rally_point_slot(payload.slot_id)?;

    let units = parse_troop_set(&payload.units)?;

    if units.units().iter().all(|value| *value == 0) && payload.hero_id.is_none() {
        return Err(ApiError::unprocessable("At least one unit is required"));
    }

    let position = Position {
        x: payload.target_x,
        y: payload.target_y,
    };
    let target_village_id = position.to_id(state.world_size);

    if let Some(target) = payload.scouting_target {
        if payload.hero_id.is_some() {
            return Err(ApiError::bad_request(
                "Heroes cannot be sent with scout-only movements",
            ));
        }

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
            .game_app
            .send_scout(SendScoutUseCaseRequest {
                player_id: user.player.id,
                source_village_id: user.village.id,
                target_village_id,
                units,
                target: scouting_target,
                attack_type,
            })
            .await
            .map_err(|err| map_application_error("action_failed", err))?;

        return Ok(Json(ActionResponse { success: true }));
    }

    match payload.movement {
        MovementKind::Attack | MovementKind::Raid => {
            let catapult_targets = parse_catapult_targets(payload.catapult_targets)?;

            state
                .game_app
                .send_attack(SendAttackUseCaseRequest {
                    player_id: user.player.id,
                    source_village_id: user.village.id,
                    target_village_id,
                    units,
                    hero_id: payload.hero_id,
                    catapult_targets,
                    attack_type: match payload.movement {
                        MovementKind::Attack => AttackType::Normal,
                        MovementKind::Raid => AttackType::Raid,
                        MovementKind::Reinforcement => AttackType::Normal,
                    },
                })
                .await
                .map_err(|err| map_application_error("action_failed", err))?;
        }
        MovementKind::Reinforcement => {
            state
                .game_app
                .send_reinforcement(SendReinforcementUseCaseRequest {
                    player_id: user.player.id,
                    source_village_id: user.village.id,
                    target_village_id,
                    units,
                    hero_id: payload.hero_id,
                })
                .await
                .map_err(|err| map_application_error("action_failed", err))?;
        }
    }

    Ok(Json(ActionResponse { success: true }))
}

#[utoipa::path(
    post,
    path = "/army/preview",
    request_body = PreviewTroopsRequest,
    responses((status = 200, body = MovementPreviewResponse))
)]
pub async fn preview_troops(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PreviewTroopsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let units = parse_troop_set(&payload.units)?;
    if units.units().iter().all(|value| *value == 0) && payload.hero_id.is_none() {
        return Err(ApiError::unprocessable("At least one unit is required"));
    }

    let selected_units = user
        .village
        .tribe
        .units()
        .iter()
        .enumerate()
        .filter_map(
            |(idx, unit)| {
                if units.get(idx) > 0 { Some(unit) } else { None }
            },
        )
        .collect::<Vec<_>>();
    let selected_hero_speed = if let Some(hero_id) = payload.hero_id {
        let hero = state
            .game_app
            .get_hero_by_player(user.player.id)
            .await
            .map_err(|err| map_application_error("unable_to_load_hero", err))?
            .ok_or_else(|| ApiError::not_found("Hero not found"))?;
        if hero.id != hero_id {
            return Err(ApiError::not_found("Hero not found"));
        }
        Some(hero.speed())
    } else {
        None
    };
    let min_speed = selected_units
        .iter()
        .map(|unit| unit.speed)
        .chain(selected_hero_speed)
        .min()
        .unwrap_or(1);
    let scout_only = payload.hero_id.is_none()
        && !selected_units.is_empty()
        && selected_units
            .iter()
            .all(|unit| matches!(unit.role, parabellum_types::army::UnitRole::Scout));
    let has_catapult_units = selected_units
        .iter()
        .any(|unit| matches!(unit.role, parabellum_types::army::UnitRole::Cata));

    let target_position = Position {
        x: payload.target_x,
        y: payload.target_y,
    };
    let travel_time_secs = user.village.position.calculate_travel_time_secs(
        target_position.clone(),
        min_speed,
        state.world_size,
        state.server_speed as u8,
    );
    let distance = user
        .village
        .position
        .distance(&target_position, state.world_size);
    let arrives_at =
        chrono::Utc::now() + chrono::Duration::seconds(std::cmp::max(1, travel_time_secs) as i64);

    // keep preview semantics aligned with send endpoint contract
    if matches!(payload.movement, MovementKind::Attack | MovementKind::Raid) {
        let _target_village_id = target_position.to_id(state.world_size);
    }

    let detected_kind = match payload.movement {
        MovementKind::Reinforcement => PreviewDetectedKind::Reinforcement,
        MovementKind::Attack | MovementKind::Raid => {
            if scout_only {
                PreviewDetectedKind::ScoutOnly
            } else {
                PreviewDetectedKind::AttackOrRaid
            }
        }
    };

    Ok(Json(MovementPreviewResponse {
        arrives_at,
        distance,
        detected_kind,
        supports_scouting_target_choice: scout_only,
        has_catapult_units,
    }))
}

/// Recalls units from a deployed army.
#[utoipa::path(
    post,
    path = "/army/recall",
    request_body = RecallTroopsRequest,
    responses((status = 200, body = ActionResponse))
)]
pub async fn recall_troops(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RecallTroopsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let units = parse_troop_set(&payload.units)?;

    state
        .game_app
        .recall_reinforcements(RecallReinforcementsUseCaseRequest {
            player_id: user.player.id,
            village_id: payload.village_id,
            army_id: payload.army_id,
            units,
            hero_id: payload.hero_id,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Releases reinforcements back to their origin village.
#[utoipa::path(
    post,
    path = "/army/release",
    request_body = ReleaseReinforcementsRequest,
    responses((status = 200, body = ActionResponse))
)]
pub async fn release_reinforcements(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ReleaseReinforcementsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let units = parse_troop_set(&payload.units)?;

    state
        .game_app
        .release_reinforcements(ReleaseReinforcementsUseCaseRequest {
            player_id: user.player.id,
            village_id: payload.village_id,
            army_id: payload.army_id,
            units,
            hero_id: payload.hero_id,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

pub async fn release_trapped_troops(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ReleaseTrappedTroopsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    state
        .game_app
        .release_trapped_troops(ReleaseTrappedTroopsUseCaseRequest {
            player_id: user.player.id,
            village_id: payload.village_id,
            army_id: payload.army_id,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

pub async fn disband_trapped_troops(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DisbandTrappedTroopsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    state
        .game_app
        .disband_trapped_troops(DisbandTrappedTroopsUseCaseRequest {
            player_id: user.player.id,
            village_id: payload.village_id,
            army_id: payload.army_id,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

pub async fn build_traps(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<BuildTrapsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    state
        .game_app
        .build_traps(BuildTrapsUseCaseRequest {
            player_id: user.player.id,
            village_id: payload.village_id,
            quantity: payload.quantity,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Cancels a pending troop movement within the 60-second cancel window.
#[utoipa::path(
    delete,
    path = "/army/movements/{movement_id}",
    params(
        ("movement_id" = Uuid, Path, description = "Troop movement id")
    ),
    responses((status = 200, body = ActionResponse))
)]
pub async fn cancel_troop_movement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(movement_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    state
        .game_app
        .cancel_troop_movement(CancelTroopMovementUseCaseRequest {
            player_id: user.player.id,
            village_id: user.village.id,
            movement_id,
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

/// Sends settlers to found a new village.
#[utoipa::path(
    post,
    path = "/map/found-village",
    request_body = FoundVillageRequest,
    responses((status = 200, body = ActionResponse))
)]
pub async fn found_village(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<FoundVillageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    state
        .game_app
        .send_settlers(SendSettlersUseCaseRequest {
            player_id: user.player.id,
            source_village_id: user.village.id,
            target_position: Position {
                x: payload.target_x,
                y: payload.target_y,
            },
            village_name: format!("{}'s Village", user.player.username),
            tribe: user.player.tribe.clone(),
        })
        .await
        .map_err(|err| map_application_error("action_failed", err))?;

    Ok(Json(ActionResponse { success: true }))
}

#[utoipa::path(
    post,
    path = "/map/found-village/preview",
    request_body = PreviewFoundVillageRequest,
    responses((status = 200, body = MovementPreviewResponse))
)]
pub async fn preview_found_village(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PreviewFoundVillageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let target_position = Position {
        x: payload.target_x,
        y: payload.target_y,
    };
    let target_field_id = target_position.to_id(state.world_size);
    let target_field = state
        .game_app
        .get_map_field(target_field_id)
        .await
        .map_err(|err| map_application_error("preview_failed", err))?;
    let target_is_empty_valley = target_field.village_id.is_none()
        && target_field.player_id.is_none()
        && matches!(
            target_field.topology,
            parabellum_game::models::map::MapFieldTopology::Valley(_)
        );
    if !target_is_empty_valley {
        return Err(ApiError::unprocessable("Target field is not available"));
    }

    let settlers_speed = user
        .village
        .tribe
        .units()
        .get(9)
        .map(|u| u.speed)
        .unwrap_or(1);
    let travel_time_secs = user.village.position.calculate_travel_time_secs(
        target_position.clone(),
        settlers_speed,
        state.world_size,
        state.server_speed as u8,
    );
    let distance = user
        .village
        .position
        .distance(&target_position, state.world_size);
    let arrives_at =
        chrono::Utc::now() + chrono::Duration::seconds(std::cmp::max(1, travel_time_secs) as i64);

    Ok(Json(MovementPreviewResponse {
        arrives_at,
        distance,
        detected_kind: PreviewDetectedKind::FoundVillage,
        supports_scouting_target_choice: false,
        has_catapult_units: false,
    }))
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
    targets: Option<Vec<CatapultTargetInput>>,
) -> Result<[Option<BuildingName>; 2], ApiError> {
    let parse_one = |input: &CatapultTargetInput| -> Result<Option<BuildingName>, ApiError> {
        match input {
            CatapultTargetInput::Building(name) => Ok(Some(name.clone())),
            CatapultTargetInput::Text(value) if value.trim().eq_ignore_ascii_case("random") => {
                Ok(None)
            }
            CatapultTargetInput::Text(_) => Err(ApiError::unprocessable(
                "catapultTargets entries must be a building name or 'random'",
            )),
        }
    };

    match targets {
        None => Ok([
            Some(BuildingName::MainBuilding),
            Some(BuildingName::Warehouse),
        ]),
        Some(values) if values.len() == 1 => {
            let first = parse_one(&values[0])?;
            Ok([first.clone(), first])
        }
        Some(values) if values.len() == 2 => Ok([parse_one(&values[0])?, parse_one(&values[1])?]),
        Some(_) => Err(ApiError::unprocessable(
            "catapultTargets must contain 1 or 2 entries",
        )),
    }
}

fn parse_resource_quantity(
    lumber: u32,
    clay: u32,
    iron: u32,
    crop: u32,
) -> Result<ResourceQuantity, ApiError> {
    let entries = [
        (ResourceKind::Lumber, lumber),
        (ResourceKind::Clay, clay),
        (ResourceKind::Iron, iron),
        (ResourceKind::Crop, crop),
    ];
    let mut non_zero = entries.into_iter().filter(|(_, quantity)| *quantity > 0);
    let Some((resource, quantity)) = non_zero.next() else {
        return Err(ApiError::unprocessable(
            "Marketplace offers require exactly one non-zero resource type.",
        ));
    };
    if non_zero.next().is_some() {
        return Err(ApiError::unprocessable(
            "Marketplace offers require exactly one non-zero resource type.",
        ));
    }
    Ok(ResourceQuantity::new(resource, quantity as u64))
}
