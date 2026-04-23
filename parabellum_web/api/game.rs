use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_app::{
    command_handlers::MarkReportReadCommandHandler,
    cqrs::{
        commands::MarkReportRead,
        queries::{
            GetLeaderboard, GetMapField, GetMapRegion, GetPlayerById, GetReportForPlayer,
            GetReportsForPlayer, GetVillageById, ListVillagesByPlayerId,
        },
    },
    queries_handlers::{
        GetLeaderboardHandler, GetMapFieldHandler, GetMapRegionHandler, GetPlayerByIdHandler,
        GetReportForPlayerHandler, GetReportsForPlayerHandler, GetVillageByIdHandler,
        ListVillagesByPlayerIdHandler,
    },
    repository::MapRegionTile,
};
use parabellum_game::models::map::MapFieldTopology;
use parabellum_types::map::{Position, ValleyTopology};

use crate::{
    api::{
        dto::{
            BootstrapResponse, LeaderboardEntryDto, PaginationDto, PlayerProfileResponse,
            PlayerVillageDto, ReportDetailResponse, ReportListItemDto, ReportsResponse,
            StatsResponse, player_summary, resources_page_response, village_list,
            village_page_response, village_summary,
        },
        errors::ApiError,
    },
    http::AppState,
    session::{CurrentUser, village_queues_or_empty},
};

use super::{authenticated_user, bearer_token};

const LEADERBOARD_PAGE_SIZE: i64 = 20;
const MAP_REGION_RADIUS: i32 = 7;

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub page: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchVillageRequest {
    pub village_id: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchVillageResponse {
    pub village_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MapRegionQuery {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub village_id: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct MapRegionResponse {
    pub center: MapPoint,
    pub radius: i32,
    pub tiles: Vec<MapTileResponse>,
}

#[derive(Debug, Serialize)]
pub struct MapPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Serialize)]
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
pub enum TileType {
    Village,
    Valley,
    Oasis,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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

pub async fn bootstrap(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;

    Ok(Json(BootstrapResponse {
        server_time: Utc::now().timestamp(),
        world_size: state.world_size,
        server_speed: state.server_speed,
        player: player_summary(&user),
        village: village_summary(&user.village),
        villages: village_list(&user),
    }))
}

pub async fn village(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let queues = village_queues_or_empty(&state, user.village.id).await;
    Ok(Json(village_page_response(&user, &queues)))
}

pub async fn resources(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let queues = village_queues_or_empty(&state, user.village.id).await;
    Ok(Json(resources_page_response(&user, &queues)))
}

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
            .map_err(|_| ApiError::unauthorized("Invalid bearer token"))?;
        state
            .token_service
            .update_refresh_session_village(
                &state.db_pool,
                claims.refresh_session_id,
                payload.village_id,
            )
            .await
            .map_err(|err| ApiError::internal(err.to_string()))?;
        let (access_token, expires_in) = state
            .token_service
            .issue_access_token_with_context(
                user.account.id,
                user.player.id,
                payload.village_id,
                claims.refresh_session_id,
            )
            .map_err(|err| ApiError::internal(err.to_string()))?;
        response.access_token = Some(access_token);
        response.expires_in = Some(expires_in);
    }

    Ok(Json(response))
}

pub async fn stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<StatsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let _user = authenticated_user(&state, &headers).await?;
    let requested_page = params.page.unwrap_or(1).max(1);

    let mut data = state
        .app_bus
        .query(
            GetLeaderboard {
                page: requested_page,
                per_page: LEADERBOARD_PAGE_SIZE,
            },
            GetLeaderboardHandler::new(),
        )
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    let total_pages = if data.total_players == 0 {
        1
    } else {
        (data.total_players + LEADERBOARD_PAGE_SIZE - 1) / LEADERBOARD_PAGE_SIZE
    };

    let mut page = requested_page;
    if data.total_players > 0 && page > total_pages {
        page = total_pages;
        data = state
            .app_bus
            .query(
                GetLeaderboard {
                    page,
                    per_page: LEADERBOARD_PAGE_SIZE,
                },
                GetLeaderboardHandler::new(),
            )
            .await
            .map_err(|err| ApiError::internal(err.to_string()))?;
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

pub async fn player_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(player_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let _user = authenticated_user(&state, &headers).await?;

    let player = state
        .app_bus
        .query(GetPlayerById { player_id }, GetPlayerByIdHandler::new())
        .await
        .map_err(|_| ApiError::not_found("Player not found"))?;

    let villages = state
        .app_bus
        .query(
            ListVillagesByPlayerId { player_id },
            ListVillagesByPlayerIdHandler::new(),
        )
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    Ok(Json(PlayerProfileResponse {
        server_time: Utc::now().timestamp(),
        player_id,
        username: player.username,
        villages: villages
            .into_iter()
            .map(|village| PlayerVillageDto {
                village_id: village.id,
                name: village.name,
                x: village.position.x,
                y: village.position.y,
                population: village.population as i32,
            })
            .collect(),
    }))
}

pub async fn reports(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let reports = state
        .app_bus
        .query(
            GetReportsForPlayer {
                player_id: user.player.id,
                limit: 50,
            },
            GetReportsForPlayerHandler::new(),
        )
        .await
        .unwrap_or_default();

    Ok(Json(ReportsResponse {
        server_time: Utc::now().timestamp(),
        reports: reports.into_iter().map(map_report_summary).collect(),
    }))
}

pub async fn report_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(report_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let report = state
        .app_bus
        .query(
            GetReportForPlayer {
                report_id,
                player_id: user.player.id,
            },
            GetReportForPlayerHandler::new(),
        )
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?
        .ok_or_else(|| ApiError::not_found("Report not found"))?;

    let _ = state
        .app_bus
        .execute(
            MarkReportRead {
                report_id,
                player_id: user.player.id,
            },
            MarkReportReadCommandHandler::new(),
        )
        .await;

    Ok(Json(ReportDetailResponse {
        server_time: Utc::now().timestamp(),
        id: report.id,
        report_type: report.report_type,
        created_at: report.created_at.timestamp(),
        payload: report.payload,
    }))
}

pub async fn map_region(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<MapRegionQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let center = determine_center(&user, &params)?;
    let center = wrap_point(center, state.world_size);

    let fields = state
        .app_bus
        .query(
            GetMapRegion {
                center: Position {
                    x: center.x,
                    y: center.y,
                },
                radius: MAP_REGION_RADIUS,
                world_size: state.world_size,
            },
            GetMapRegionHandler::new(),
        )
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    Ok(Json(MapRegionResponse {
        center,
        radius: MAP_REGION_RADIUS,
        tiles: fields.into_iter().map(MapTileResponse::from).collect(),
    }))
}

pub async fn map_field(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(field_id): Path<u32>,
) -> Result<impl IntoResponse, ApiError> {
    let _user = authenticated_user(&state, &headers).await?;
    let field = state
        .app_bus
        .query(GetMapField { field_id }, GetMapFieldHandler::new())
        .await
        .map_err(|_| ApiError::not_found("Map field not found"))?;

    let village = match field.village_id {
        Some(village_id) => state
            .app_bus
            .query(
                GetVillageById { id: village_id },
                GetVillageByIdHandler::new(),
            )
            .await
            .ok(),
        None => None,
    };

    let player_name = match field.player_id {
        Some(player_id) => state
            .app_bus
            .query(GetPlayerById { player_id }, GetPlayerByIdHandler::new())
            .await
            .ok()
            .map(|player| player.username),
        None => None,
    };

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
        village_name: village.as_ref().map(|it| it.name.clone()),
        player_name,
        village_population: village.map(|it| it.population as i32),
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

fn map_report_summary(report: parabellum_app::cqrs::queries::ReportView) -> ReportListItemDto {
    ReportListItemDto {
        id: report.id,
        report_type: report.report_type,
        payload: report.payload,
        created_at: report.created_at.timestamp(),
        is_read: report.read_at.is_some(),
    }
}
