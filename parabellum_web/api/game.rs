//! Read-oriented game and profile handlers.
//!
//! These handlers expose canonical API data used by the SPA:
//! - current user context (`/me/*`)
//! - game hydration (`/game/context`)
//! - map, reports, stats and player profile reads

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use parabellum_app::read_models::MapRegionTile;
use parabellum_game::models::map::MapFieldTopology;
use parabellum_types::buildings::BuildingName;
use parabellum_types::map::ValleyTopology;

use crate::{
    api::{
        dto::{
            GameContextResponse, LeaderboardEntryDto, PaginationDto, PlayerProfileResponse,
            PlayerVillageDto, ReportDetailPayloadResponse, ReportDetailResponse, ReportListItemDto,
            ReportsResponse, StatsResponse, game_context_response, session_user,
        },
        errors::ApiError,
    },
    http::AppState,
    session::CurrentUser,
};

use super::error_mapping::{internal_error, map_application_error};
use super::helpers::map_token_error;
use super::{authenticated_user, bearer_token};

const LEADERBOARD_PAGE_SIZE: i64 = 20;
const MAP_REGION_RADIUS: i32 = 7;

#[derive(Debug, Deserialize, ToSchema)]
/// Pagination query for leaderboard endpoint.
pub struct StatsQuery {
    pub page: Option<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReportsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Request payload to switch current village.
pub struct SwitchVillageRequest {
    pub village_id: u32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Response for village switch operation.
pub struct SwitchVillageResponse {
    pub village_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Session endpoint response for authenticated clients.
pub struct MeSessionResponse {
    pub authenticated: bool,
    pub user: crate::api::dto::SessionUserDto,
    pub current_village_id: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
/// Query params for map region retrieval.
pub struct MapRegionQuery {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub village_id: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
/// Response for map region endpoint.
pub struct MapRegionResponse {
    pub center: MapPoint,
    pub radius: i32,
    pub tiles: Vec<MapTileResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
/// 2D point on world map.
pub struct MapPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Serialize, ToSchema)]
/// Tile data returned by map region endpoint.
pub struct MapTileResponse {
    pub x: i32,
    pub y: i32,
    pub field_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub village_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub village_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub village_population: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_capital: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tribe: Option<String>,
    pub tile_type: TileType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valley: Option<ValleyDistribution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oasis: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
/// Runtime tile category.
pub enum TileType {
    Village,
    Valley,
    Oasis,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Detailed payload for a specific map field.
pub struct MapFieldDetailResponse {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub tile_type: TileType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub village_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub village_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub village_population: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_capital: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valley: Option<ValleyDistribution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oasis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oasis_bonus: Option<ValleyDistribution>,
    pub can_preview_founding: bool,
    pub has_marketplace: bool,
    pub has_rally_point: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marketplace_slot_id: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rally_point_slot_id: Option<u8>,
}

#[derive(Debug, Serialize, ToSchema)]
/// Valley resource distribution for valley/oasis map payloads.
pub struct ValleyDistribution {
    pub lumber: u8,
    pub clay: u8,
    pub iron: u8,
    pub crop: u8,
}

impl From<ValleyTopology> for ValleyDistribution {
    fn from(valley: ValleyTopology) -> Self {
        Self {
            lumber: valley.0,
            clay: valley.1,
            iron: valley.2,
            crop: valley.3,
        }
    }
}

impl From<MapRegionTile> for MapTileResponse {
    fn from(tile: MapRegionTile) -> Self {
        let field = tile.field;
        let (tile_type, valley, oasis) = match field.topology {
            MapFieldTopology::Oasis(variant) => {
                (TileType::Oasis, None, Some(format!("{variant:?}")))
            }
            MapFieldTopology::Valley(valley) => {
                if field.village_id.is_some() {
                    (TileType::Village, Some(valley.into()), None)
                } else {
                    (TileType::Valley, Some(valley.into()), None)
                }
            }
        };

        Self {
            x: field.position.x,
            y: field.position.y,
            field_id: field.id,
            village_id: field.village_id,
            player_id: field.player_id,
            village_name: tile.village_name,
            village_population: tile.village_population,
            is_capital: tile.is_capital,
            player_name: tile.player_name,
            tribe: tile.tribe.map(|t| format!("{t:?}")),
            tile_type,
            valley,
            oasis,
        }
    }
}

/// Returns current authenticated session user and active village id.
#[utoipa::path(
    get,
    path = "/me/session",
    responses((status = 200, body = MeSessionResponse))
)]
pub async fn me_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(MeSessionResponse {
        authenticated: true,
        user: session_user(&user),
        current_village_id: user.village.id,
    }))
}

/// Returns the canonical SPA game hydration payload.
#[utoipa::path(
    get,
    path = "/game/context",
    responses((status = 200, body = GameContextResponse))
)]
pub async fn game_context(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let village = user.village.clone();
    let unread_reports_count = state
        .game_app
        .count_unread_reports_for_player(user.player.id)
        .await
        .map_err(|err| map_application_error("unable_to_count_unread_reports", err))?;
    let army_state = state
        .game_app
        .get_village_army_state_view(village.id)
        .await
        .map_err(|e| map_application_error("unable_to_load_village_army_state", e))?;
    let queues = state
        .game_app
        .get_village_queues(village.id)
        .await
        .map_err(|e| map_application_error("unable_to_load_village_queues", e))?;
    let movements = state
        .game_app
        .get_village_troop_movements(village.id)
        .await
        .map_err(|e| map_application_error("unable_to_load_village_troop_movements", e))?;
    Ok(Json(game_context_response(
        Utc::now().timestamp(),
        state.world_size,
        state.server_speed,
        unread_reports_count,
        &user,
        &village,
        &queues,
        &army_state,
        &movements,
    )))
}

/// Switches current village and rotates access token context when bearer is present.
#[utoipa::path(
    post,
    path = "/me/village/current",
    request_body = SwitchVillageRequest,
    responses((status = 200, body = SwitchVillageResponse))
)]
pub async fn switch_village(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SwitchVillageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    if !user
        .villages
        .iter()
        .any(|v| v.id == payload.village_id && v.player_id == user.player.id)
    {
        return Err(ApiError::not_found(
            "Village not available for the current player",
        ));
    }

    let mut response = SwitchVillageResponse {
        village_id: payload.village_id,
        access_token: None,
        expires_in: None,
    };

    if let Some(token) = bearer_token(&headers) {
        let claims = state
            .token_service
            .verify_access_token(token)
            .map_err(map_token_error)?;
        state
            .token_service
            .update_refresh_session_village(
                &state.db_pool,
                claims.refresh_session_id,
                payload.village_id,
            )
            .await
            .map_err(|err| internal_error("unable_to_update_refresh_session_village", err))?;
        let (access_token, expires_in) = state
            .token_service
            .issue_access_token_with_context(
                user.account.id,
                user.player.id,
                payload.village_id,
                claims.refresh_session_id,
            )
            .map_err(|err| internal_error("unable_to_issue_access_token", err))?;
        response.access_token = Some(access_token);
        response.expires_in = Some(expires_in);
    }

    Ok(Json(response))
}

/// Returns paginated leaderboard data.
#[utoipa::path(
    get,
    path = "/stats",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based)")
    ),
    responses((status = 200, body = StatsResponse))
)]
pub async fn stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<StatsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let _user = authenticated_user(&state, &headers).await?;
    let requested_page = params.page.unwrap_or(1).max(1);

    let mut data = state
        .game_app
        .get_leaderboard_page(requested_page, LEADERBOARD_PAGE_SIZE)
        .await
        .map_err(|err| map_application_error("unable_to_load_leaderboard", err))?;

    let total_pages = if data.total_players == 0 {
        1
    } else {
        (data.total_players + LEADERBOARD_PAGE_SIZE - 1) / LEADERBOARD_PAGE_SIZE
    };

    let mut page = requested_page;
    if data.total_players > 0 && page > total_pages {
        page = total_pages;
        data = state
            .game_app
            .get_leaderboard_page(page, LEADERBOARD_PAGE_SIZE)
            .await
            .map_err(|err| map_application_error("unable_to_load_leaderboard", err))?;
    }

    Ok(Json(StatsResponse {
        server_time: Utc::now().timestamp(),
        entries: data
            .entries
            .into_iter()
            .enumerate()
            .map(|(idx, entry)| LeaderboardEntryDto {
                player_id: entry.player_id.to_string(),
                rank: (page - 1) * LEADERBOARD_PAGE_SIZE + idx as i64 + 1,
                username: entry.username,
                tribe: format!("{:?}", entry.tribe),
                village_count: entry.village_count,
                population: entry.population,
            })
            .collect(),
        pagination: PaginationDto {
            page,
            per_page: LEADERBOARD_PAGE_SIZE,
            total_players: data.total_players,
            total_pages: total_pages.max(1),
        },
    }))
}

/// Returns profile summary for a specific player.
#[utoipa::path(
    get,
    path = "/players/{id}",
    params(
        ("id" = Uuid, Path, description = "Player id")
    ),
    responses((status = 200, body = PlayerProfileResponse))
)]
pub async fn player_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(player_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    let player = state
        .game_app
        .get_player_by_id(player_id)
        .await
        .map_err(|err| map_application_error("unable_to_load_player_profile", err))?;

    let villages = state
        .game_app
        .list_villages_by_player_id(player_id)
        .await
        .map_err(|err| map_application_error("unable_to_load_player_villages", err))?;

    Ok(Json(PlayerProfileResponse {
        server_time: Utc::now().timestamp(),
        player_id,
        username: player.username,
        villages: villages
            .into_iter()
            .map(|village| PlayerVillageDto {
                village_id: village.village_id,
                name: village.village_name,
                x: village.position.x,
                y: village.position.y,
                population: village.population as i32,
                is_capital: village.is_capital,
                distance_from_current: user
                    .village
                    .position
                    .distance(&village.position, state.world_size),
            })
            .collect(),
    }))
}

/// Returns recent reports for current player.
#[utoipa::path(
    get,
    path = "/reports",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based)"),
        ("per_page" = Option<i64>, Query, description = "Items per page (max 100)")
    ),
    responses((status = 200, body = ReportsResponse))
)]
pub async fn reports(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ReportsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let reports = state
        .game_app
        .list_reports_for_player(user.player.id, offset, per_page + 1)
        .await
        .map_err(|err| map_application_error("unable_to_list_reports", err))?;
    let has_more = reports.len() as i64 > per_page;
    let reports: Vec<_> = reports.into_iter().take(per_page as usize).collect();

    Ok(Json(ReportsResponse {
        server_time: Utc::now().timestamp(),
        reports: reports.into_iter().map(map_report_summary).collect(),
        pagination: crate::api::dto::ReportsPaginationDto {
            page,
            per_page,
            has_more,
        },
    }))
}

/// Returns full report payload and marks report as read.
#[utoipa::path(
    get,
    path = "/reports/{id}",
    params(
        ("id" = Uuid, Path, description = "Report id")
    ),
    responses((status = 200, body = ReportDetailPayloadResponse))
)]
pub async fn report_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(report_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let report = state
        .game_app
        .get_report_for_player(report_id, user.player.id)
        .await
        .map_err(|err| map_application_error("unable_to_load_report", err))?
        .ok_or_else(|| ApiError::not_found("Report not found"))?;

    let _ = state
        .game_app
        .mark_report_as_read(report_id, user.player.id)
        .await;

    Ok(Json(ReportDetailResponse {
        server_time: Utc::now().timestamp(),
        id: report.id,
        report_type: report.report_type,
        created_at: report.created_at.timestamp(),
        payload: report.payload,
    }))
}

/// Returns wrapped map region around requested/default center.
#[utoipa::path(
    get,
    path = "/map/region",
    params(
        ("x" = Option<i32>, Query, description = "Center x coordinate"),
        ("y" = Option<i32>, Query, description = "Center y coordinate"),
        ("village_id" = Option<u32>, Query, description = "Use owned village as center")
    ),
    responses((status = 200, body = MapRegionResponse))
)]
pub async fn map_region(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<MapRegionQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let center = determine_center(&user, &params)?;
    let center = wrap_point(center, state.world_size);

    let fields = state
        .game_app
        .get_map_region(center.x, center.y, MAP_REGION_RADIUS, state.world_size)
        .await
        .map_err(|err| map_application_error("unable_to_load_map_region", err))?;

    Ok(Json(MapRegionResponse {
        center,
        radius: MAP_REGION_RADIUS,
        tiles: fields.into_iter().map(MapTileResponse::from).collect(),
    }))
}

/// Returns details for one map field.
#[utoipa::path(
    get,
    path = "/map/fields/{id}",
    params(
        ("id" = u32, Path, description = "Map field id")
    ),
    responses((status = 200, body = MapFieldDetailResponse))
)]
pub async fn map_field(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(field_id): Path<u32>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let field = state
        .game_app
        .get_map_field(field_id)
        .await
        .map_err(|err| map_application_error("unable_to_load_map_field", err))?;
    let region_tile = state
        .game_app
        .get_map_region_tile_by_field_id(field_id)
        .await
        .map_err(|err| map_application_error("unable_to_load_map_field_tile", err))?;

    let (tile_type, valley, oasis, oasis_bonus) = match field.topology {
        MapFieldTopology::Oasis(variant) => {
            let (lumber, clay, iron, crop) = match variant {
                parabellum_types::map::OasisTopology::Lumber => (25, 0, 0, 0),
                parabellum_types::map::OasisTopology::LumberCrop => (25, 0, 0, 25),
                parabellum_types::map::OasisTopology::Clay => (0, 25, 0, 0),
                parabellum_types::map::OasisTopology::ClayCrop => (0, 25, 0, 25),
                parabellum_types::map::OasisTopology::Iron => (0, 0, 25, 0),
                parabellum_types::map::OasisTopology::IronCrop => (0, 0, 25, 25),
                parabellum_types::map::OasisTopology::Crop => (0, 0, 0, 25),
                parabellum_types::map::OasisTopology::Crop50 => (0, 0, 0, 50),
            };
            (
                TileType::Oasis,
                None,
                Some(format!("{variant:?}")),
                Some(ValleyDistribution {
                    lumber,
                    clay,
                    iron,
                    crop,
                }),
            )
        }
        MapFieldTopology::Valley(valley) => {
            let tile_type = if field.village_id.is_some() {
                TileType::Village
            } else {
                TileType::Valley
            };
            (tile_type, Some(valley.into()), None, None)
        }
    };

    let marketplace_slot = user
        .village
        .get_building_by_name(&BuildingName::Marketplace);
    let rally_point_slot = user.village.get_building_by_name(&BuildingName::RallyPoint);
    let has_marketplace = marketplace_slot.is_some();
    let has_rally_point = rally_point_slot.is_some();

    let can_preview_founding =
        if matches!(tile_type, TileType::Valley) && field.village_id.is_none() {
            let settlers_ready = user.village.count_settlers_at_home() >= 3;
            let has_rally = has_rally_point;
            let has_resources =
                user.village
                    .has_enough_resources(&parabellum_types::common::ResourceGroup::new(
                        800, 800, 800, 800,
                    ));
            let expansion_info = state
                .game_app
                .get_expansion_culture_info(user.player.id, user.village.id, state.server_speed)
                .await
                .ok();
            let cp_ok = expansion_info
                .as_ref()
                .map(|info| info.player_culture_points >= info.next_cp_required)
                .unwrap_or(false);
            let owned_villages = state
                .game_app
                .list_villages_by_player_id(user.player.id)
                .await
                .ok();
            let child_villages_count = owned_villages
                .as_ref()
                .map(|villages| {
                    villages
                        .iter()
                        .filter(|v| v.parent_village_id == Some(user.village.id))
                        .count() as u8
                })
                .unwrap_or(0);
            let free_slot = user
                .village
                .max_foundation_slots()
                .saturating_sub(child_villages_count)
                > 0;
            settlers_ready && has_rally && has_resources && cp_ok && free_slot
        } else {
            false
        };

    Ok(Json(MapFieldDetailResponse {
        id: field.id,
        x: field.position.x,
        y: field.position.y,
        tile_type,
        village_id: field.village_id,
        player_id: field.player_id,
        village_name: region_tile.as_ref().and_then(|t| t.village_name.clone()),
        player_name: region_tile.as_ref().and_then(|t| t.player_name.clone()),
        village_population: region_tile.as_ref().and_then(|t| t.village_population),
        is_capital: region_tile.as_ref().and_then(|t| t.is_capital),
        valley,
        oasis,
        oasis_bonus,
        can_preview_founding,
        has_marketplace,
        has_rally_point,
        marketplace_slot_id: marketplace_slot.map(|slot| slot.slot_id),
        rally_point_slot_id: rally_point_slot.map(|slot| slot.slot_id),
    }))
}

fn determine_center(user: &CurrentUser, params: &MapRegionQuery) -> Result<MapPoint, ApiError> {
    if let Some(village_id) = params.village_id {
        if let Some(village) = user.villages.iter().find(|v| v.id == village_id) {
            return Ok(MapPoint {
                x: village.position.x,
                y: village.position.y,
            });
        }

        return Err(ApiError::bad_request(
            "Unknown village id for current player",
        ));
    }

    match (params.x, params.y) {
        (Some(x), Some(y)) => Ok(MapPoint { x, y }),
        (None, None) => Ok(MapPoint {
            x: user.village.position.x,
            y: user.village.position.y,
        }),
        _ => Err(ApiError::bad_request(
            "Both x and y coordinates are required",
        )),
    }
}

fn wrap_point(point: MapPoint, world_size: i32) -> MapPoint {
    MapPoint {
        x: wrap_coordinate(point.x, world_size),
        y: wrap_coordinate(point.y, world_size),
    }
}

fn wrap_coordinate(value: i32, world_size: i32) -> i32 {
    if world_size <= 0 {
        return value;
    }

    let span = world_size * 2 + 1;
    let mut normalized = (value + world_size) % span;
    if normalized < 0 {
        normalized += span;
    }
    normalized - world_size
}

fn map_report_summary(report: parabellum_app::villages::models::ReportModel) -> ReportListItemDto {
    ReportListItemDto {
        id: report.id,
        report_type: report.report_type,
        payload: report.payload,
        created_at: report.created_at.timestamp(),
        is_read: report.read_at.is_some(),
    }
}
