//! Read-oriented game and profile handlers.
//!
//! These handlers expose canonical API data used by the SPA:
//! - current user context (`/me/*`)
//! - village resource/overview snapshots (`/villages/{id}/*`)
//! - map, reports, stats and player profile reads

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_app::read_models::MapRegionTile;
use parabellum_game::models::map::MapFieldTopology;
use parabellum_game::models::village::Village;
use parabellum_types::map::ValleyTopology;

use crate::{
    api::{
        dto::{
            LeaderboardEntryDto, MeContextResponse, PaginationDto, PlayerProfileResponse,
            PlayerVillageDto, ReportDetailResponse, ReportListItemDto, ReportsResponse,
            StatsResponse, player_summary, session_user, village_list, village_overview_response,
            village_resources_response, village_summary,
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

#[derive(Debug, Deserialize)]
/// Pagination query for leaderboard endpoint.
pub struct StatsQuery {
    pub page: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Request payload to switch current village.
pub struct SwitchVillageRequest {
    pub village_id: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Response for village switch operation.
pub struct SwitchVillageResponse {
    pub village_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Session endpoint response for authenticated clients.
pub struct MeSessionResponse {
    pub authenticated: bool,
    pub user: crate::api::dto::SessionUserDto,
    pub current_village_id: u32,
}

#[derive(Debug, Deserialize)]
/// Query params for map region retrieval.
pub struct MapRegionQuery {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub village_id: Option<u32>,
}

#[derive(Debug, Serialize)]
/// Response for map region endpoint.
pub struct MapRegionResponse {
    pub center: MapPoint,
    pub radius: i32,
    pub tiles: Vec<MapTileResponse>,
}

#[derive(Debug, Serialize)]
/// 2D point on world map.
pub struct MapPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Serialize)]
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
    pub player_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tribe: Option<String>,
    pub tile_type: TileType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valley: Option<ValleyDistribution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oasis: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
/// Runtime tile category.
pub enum TileType {
    Village,
    Valley,
    Oasis,
}

#[derive(Debug, Serialize)]
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
    pub valley: Option<ValleyDistribution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oasis: Option<String>,
}

#[derive(Debug, Serialize)]
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
            player_name: tile.player_name,
            tribe: tile.tribe.map(|t| format!("{t:?}")),
            tile_type,
            valley,
            oasis,
        }
    }
}

/// Returns current authenticated session user and active village id.
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

/// Returns authenticated user context for SPA shell state.
pub async fn me_context(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(MeContextResponse {
        server_time: Utc::now().timestamp(),
        world_size: state.world_size,
        server_speed: state.server_speed,
        player: player_summary(&user),
        current_village: village_summary(&user.village),
        villages: village_list(&user),
    }))
}

/// Returns village overview (building slots + queue) for owned village.
pub async fn village_overview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(village_id): Path<u32>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let village = owned_village_by_id(&state, &user, village_id).await?;
    let queues = state
        .game_app
        .get_village_queues(village.id)
        .await
        .map_err(|e| map_application_error("unable_to_load_village_queues", e))?;
    Ok(Json(village_overview_response(&village, &queues)))
}

/// Returns village resource fields and queue for owned village.
pub async fn village_resources(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(village_id): Path<u32>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let village = owned_village_by_id(&state, &user, village_id).await?;
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
    Ok(Json(village_resources_response(&village, &queues, &army_state)))
}

/// Switches current village and rotates access token context when bearer is present.
pub async fn switch_village(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SwitchVillageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    if !user
        .villages
        .iter()
        .any(|village| village.id == payload.village_id)
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
pub async fn player_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(player_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let _user = authenticated_user(&state, &headers).await?;

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
            })
            .collect(),
    }))
}

/// Returns recent reports for current player.
pub async fn reports(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let reports = state
        .game_app
        .list_reports_for_player(user.player.id, 50)
        .await
        .map_err(|err| map_application_error("unable_to_list_reports", err))?;

    Ok(Json(ReportsResponse {
        server_time: Utc::now().timestamp(),
        reports: reports.into_iter().map(map_report_summary).collect(),
    }))
}

/// Returns full report payload and marks report as read.
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
pub async fn map_field(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(field_id): Path<u32>,
) -> Result<impl IntoResponse, ApiError> {
    let _user = authenticated_user(&state, &headers).await?;
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

    let (tile_type, valley, oasis) = match field.topology {
        MapFieldTopology::Oasis(variant) => (TileType::Oasis, None, Some(format!("{variant:?}"))),
        MapFieldTopology::Valley(valley) => {
            let tile_type = if field.village_id.is_some() {
                TileType::Village
            } else {
                TileType::Valley
            };
            (tile_type, Some(valley.into()), None)
        }
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
        village_population: region_tile.and_then(|t| t.village_population),
        valley,
        oasis,
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

async fn owned_village_by_id(
    state: &AppState,
    user: &CurrentUser,
    village_id: u32,
) -> Result<Village, ApiError> {
    let village = state
        .game_app
        .get_village_model(village_id)
        .await
        .map_err(|e| map_application_error("unable_to_load_village", e))?;
    if village.player_id != user.player.id {
        return Err(ApiError::not_found(
            "Village not available for the current player",
        ));
    }
    Ok(village.into())
}
