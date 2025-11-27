use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    handlers::{CurrentUser, render_template},
    http::AppState,
    templates::MapTemplate,
};
use parabellum_app::{cqrs::queries::GetMapRegion, queries_handlers::GetMapRegionHandler};
use parabellum_game::models::map::{MapField, MapFieldTopology};
use parabellum_types::map::{Position, ValleyTopology};

const MAP_REGION_RADIUS: i32 = 7;

pub async fn map(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
    let template = MapTemplate {
        current_user: Some(user),
        nav_active: "map",
        world_size: state.world_size,
    };
    render_template(template, None).into_response()
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

impl From<MapField> for MapTileResponse {
    fn from(field: MapField) -> Self {
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
            tile_type,
            valley,
            oasis,
        }
    }
}

pub async fn map_region(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Query(params): Query<MapRegionQuery>,
) -> Response {
    let center = match determine_center(&current_user, &params) {
        Ok(center) => center,
        Err(response) => return response,
    };
    let center = wrap_point(center, state.world_size);

    let query = GetMapRegion {
        center: Position {
            x: center.x,
            y: center.y,
        },
        radius: MAP_REGION_RADIUS,
        world_size: state.world_size,
    };

    let fields = match state.app_bus.query(query, GetMapRegionHandler::new()).await {
        Ok(fields) => fields,
        Err(e) => {
            tracing::error!("Unable to load map region: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let tiles = fields.into_iter().map(MapTileResponse::from).collect();

    let response = MapRegionResponse {
        center,
        radius: MAP_REGION_RADIUS,
        tiles,
    };
    Json(response).into_response()
}

fn determine_center(user: &CurrentUser, params: &MapRegionQuery) -> Result<MapPoint, Response> {
    if let Some(village_id) = params.village_id {
        if let Some(village) = user.villages.iter().find(|v| v.id == village_id) {
            return Ok(MapPoint {
                x: village.position.x,
                y: village.position.y,
            });
        } else {
            return Err((
                StatusCode::BAD_REQUEST,
                "Unknown village id for current player",
            )
                .into_response());
        }
    }

    match (params.x, params.y) {
        (Some(x), Some(y)) => Ok(MapPoint { x, y }),
        (None, None) => Ok(MapPoint {
            x: user.village.position.x,
            y: user.village.position.y,
        }),
        _ => Err((
            StatusCode::BAD_REQUEST,
            "Both x and y coordinates are required",
        )
            .into_response()),
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
