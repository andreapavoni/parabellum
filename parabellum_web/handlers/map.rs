use crate::{
    components::{PageLayout, wrap_in_html},
    handlers::helpers::{CsrfForm, CurrentUser, HasCsrfToken, create_layout_data, generate_csrf},
    http::AppState,
    pages::MapPage,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use dioxus::prelude::*;
use parabellum_app::{
    command_handlers::FoundVillageCommandHandler,
    cqrs::{
        commands::FoundVillage,
        queries::{GetMapField, GetMapRegion, GetVillageById, ListVillagesByPlayerId},
    },
    queries_handlers::{
        GetMapFieldHandler, GetMapRegionHandler, GetVillageByIdHandler,
        ListVillagesByPlayerIdHandler,
    },
    repository::MapRegionTile,
};
use parabellum_game::models::{culture_points, map::MapFieldTopology};
use parabellum_types::{
    army::TroopSet,
    map::{Position, ValleyTopology},
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, MapAccess, Visitor},
};
use uuid::Uuid;

const MAP_REGION_RADIUS: i32 = 7;

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
struct ErrorResponse {
    message: String,
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

/// GET /map - Render the map page using Dioxus SSR.
pub async fn map_page(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
    let layout_data = create_layout_data(&user, "map");

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data.clone(),
            MapPage {
                village: layout_data.village.unwrap(),
                world_size: state.world_size
            }
        }
    });

    Html(wrap_in_html(&body_content))
}

/// GET /map/{field_id} - map centered on a specific village/valley id
pub async fn map_page_with_id(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(field_id): Path<u32>,
) -> impl IntoResponse {
    let layout_data = create_layout_data(&user, "map");

    // Try to load the requested village; fall back to the user's current village on failure
    let target_village = state
        .app_bus
        .query(
            GetVillageById { id: field_id },
            GetVillageByIdHandler::new(),
        )
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(
                "Unable to load village {} for map view, using current village. Error: {}",
                field_id,
                e
            );
            layout_data
                .village
                .clone()
                .expect("layout_data always provides a village")
        });

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data.clone(),
            MapPage {
                village: target_village,
                world_size: state.world_size
            }
        }
    });

    Html(wrap_in_html(&body_content))
}

/// GET /map/field/{id} - Render detailed info about a specific map field (valley/village/oasis)
pub async fn map_field_page(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(field_id): Path<u32>,
    jar: axum_extra::extract::SignedCookieJar,
) -> Response {
    // Fetch the map field
    let field = match state
        .app_bus
        .query(GetMapField { field_id }, GetMapFieldHandler::new())
        .await
    {
        Ok(field) => field,
        Err(e) => {
            tracing::error!("Unable to load map field {}: {}", field_id, e);
            return (StatusCode::NOT_FOUND, "Map field not found").into_response();
        }
    };

    let layout_data = create_layout_data(&user, "map");

    // Get player's village count for CP calculation
    let player_villages = match state
        .app_bus
        .query(
            ListVillagesByPlayerId {
                player_id: user.player.id,
            },
            ListVillagesByPlayerIdHandler::new(),
        )
        .await
    {
        Ok(villages) => villages,
        Err(e) => {
            tracing::error!("Unable to load player villages: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to load player data",
            )
                .into_response();
        }
    };

    let village_count = player_villages.len();
    let speed = parabellum_types::common::Speed::from(state.server_speed);
    let required_cp = culture_points::required_cp(speed, village_count + 1);

    // Determine if the current village has enough culture points and settlers to found
    let can_found_village = user.player.culture_points >= required_cp
        && user
            .village
            .army()
            .map(|army| {
                let settler_idx = user
                    .village
                    .tribe
                    .get_unit_idx_by_name(&parabellum_types::army::UnitName::Settler)
                    .unwrap_or(9);
                army.units().get(settler_idx) >= 3
            })
            .unwrap_or(false);

    // Render different components based on tile type
    match field.topology {
        MapFieldTopology::Valley(valley) => {
            if field.village_id.is_some() {
                // This valley has a village on it - show village info
                let village = match state
                    .app_bus
                    .query(
                        GetVillageById { id: field_id },
                        GetVillageByIdHandler::new(),
                    )
                    .await
                {
                    Ok(v) => v,
                    Err(_) => {
                        return (StatusCode::NOT_FOUND, "Village not found").into_response();
                    }
                };

                let (jar, csrf_token) = super::helpers::generate_csrf(jar);

                let body_content = dioxus_ssr::render_element(rsx! {
                    PageLayout {
                        data: layout_data.clone(),
                        crate::pages::MapFieldVillagePage {
                            village: village,
                            current_village_id: user.village.id,
                            csrf_token: csrf_token,
                        }
                    }
                });

                (jar, Html(wrap_in_html(&body_content))).into_response()
            } else {
                // Empty valley - show valley info with optional "Found Village" button
                let (jar, csrf_token) = super::helpers::generate_csrf(jar);

                let body_content = dioxus_ssr::render_element(rsx! {
                    PageLayout {
                        data: layout_data.clone(),
                        crate::pages::MapFieldValleyPage {
                            field_id: field_id,
                            position: field.position.clone(),
                            valley: valley,
                            can_found_village: can_found_village,
                            current_village_id: user.village.id,
                            current_village_name: user.village.name.clone(),
                            csrf_token: csrf_token,
                        }
                    }
                });

                (jar, Html(wrap_in_html(&body_content))).into_response()
            }
        }
        MapFieldTopology::Oasis(_oasis) => {
            // Oasis info page
            let (jar, csrf_token) = super::helpers::generate_csrf(jar);

            let body_content = dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data.clone(),
                    crate::pages::MapFieldOasisPage {
                        field_id: field_id,
                        position: field.position.clone(),
                        csrf_token: csrf_token,
                    }
                }
            });

            (jar, Html(wrap_in_html(&body_content))).into_response()
        }
    }
}

/// GET /map?x={x}&y={y}
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
            return map_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to load map region",
            );
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
            return Err(map_error(
                StatusCode::BAD_REQUEST,
                "Unknown village id for current player",
            ));
        }
    }

    match (params.x, params.y) {
        (Some(x), Some(y)) => Ok(MapPoint { x, y }),
        (None, None) => Ok(MapPoint {
            x: user.village.position.x,
            y: user.village.position.y,
        }),
        _ => Err(map_error(
            StatusCode::BAD_REQUEST,
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

fn map_error(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(ErrorResponse {
            message: message.into(),
        }),
    )
        .into_response()
}

#[derive(Debug)]
pub struct FoundVillageConfirmForm {
    pub field_id: u32,
    pub target_x: i32,
    pub target_y: i32,
    pub csrf_token: String,
}

impl<'de> Deserialize<'de> for FoundVillageConfirmForm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier)]
        enum Field {
            #[serde(rename = "field_id")]
            FieldId,
            #[serde(rename = "target_x")]
            TargetX,
            #[serde(rename = "target_y")]
            TargetY,
            #[serde(rename = "csrf_token")]
            CsrfToken,
        }

        struct FormVisitor;

        impl<'de> Visitor<'de> for FormVisitor {
            type Value = FoundVillageConfirmForm;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("found village confirm form data")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut field_id = None;
                let mut target_x = None;
                let mut target_y = None;
                let mut csrf_token = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::FieldId => {
                            if field_id.is_some() {
                                return Err(de::Error::duplicate_field("field_id"));
                            }
                            field_id = Some(map.next_value()?);
                        }
                        Field::TargetX => {
                            if target_x.is_some() {
                                return Err(de::Error::duplicate_field("target_x"));
                            }
                            target_x = Some(map.next_value()?);
                        }
                        Field::TargetY => {
                            if target_y.is_some() {
                                return Err(de::Error::duplicate_field("target_y"));
                            }
                            target_y = Some(map.next_value()?);
                        }
                        Field::CsrfToken => {
                            if csrf_token.is_some() {
                                return Err(de::Error::duplicate_field("csrf_token"));
                            }
                            csrf_token = Some(map.next_value()?);
                        }
                    }
                }

                let field_id = field_id.ok_or_else(|| de::Error::missing_field("field_id"))?;
                let target_x = target_x.ok_or_else(|| de::Error::missing_field("target_x"))?;
                let target_y = target_y.ok_or_else(|| de::Error::missing_field("target_y"))?;
                let csrf_token =
                    csrf_token.ok_or_else(|| de::Error::missing_field("csrf_token"))?;

                Ok(FoundVillageConfirmForm {
                    field_id,
                    target_x,
                    target_y,
                    csrf_token,
                })
            }
        }

        deserializer.deserialize_map(FormVisitor)
    }
}

impl HasCsrfToken for FoundVillageConfirmForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

/// POST /map/field/{id}/found/confirm - Show confirmation page for founding a village
pub async fn found_village_confirm(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(field_id): Path<u32>,
    CsrfForm { jar, .. }: CsrfForm<FoundVillageConfirmForm>,
) -> Response {
    // Fetch the map field
    let field = match state
        .app_bus
        .query(GetMapField { field_id }, GetMapFieldHandler::new())
        .await
    {
        Ok(field) => field,
        Err(e) => {
            tracing::error!("Unable to load map field {}: {}", field_id, e);
            return (StatusCode::NOT_FOUND, "Map field not found").into_response();
        }
    };

    // Ensure it's a valley (not an oasis)
    match field.topology {
        MapFieldTopology::Valley(_) => (),
        MapFieldTopology::Oasis(_) => {
            return (StatusCode::BAD_REQUEST, "Cannot found village on an oasis").into_response();
        }
    };

    // Ensure it's an empty valley
    if field.village_id.is_some() {
        return (StatusCode::BAD_REQUEST, "This valley already has a village").into_response();
    }

    // Get settler unit index
    let settler_idx = user
        .village
        .tribe
        .get_unit_idx_by_name(&parabellum_types::army::UnitName::Settler)
        .unwrap_or(9);

    let mut settlers = TroopSet::default();
    settlers.set(settler_idx, 3);

    // Get player's village count for CP calculation
    let player_villages = match state
        .app_bus
        .query(
            ListVillagesByPlayerId {
                player_id: user.player.id,
            },
            ListVillagesByPlayerIdHandler::new(),
        )
        .await
    {
        Ok(villages) => villages,
        Err(e) => {
            tracing::error!("Unable to load player villages: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to load player data",
            )
                .into_response();
        }
    };

    let village_count = player_villages.len();
    let speed = parabellum_types::common::Speed::from(state.server_speed);
    let required_cp = culture_points::required_cp(speed, village_count + 1);

    // Verify the account has the required culture points and village has settlers
    let can_found = user.player.culture_points >= required_cp
        && user
            .village
            .army()
            .map(|army| army.units().get(settler_idx) >= 3)
            .unwrap_or(false);

    if !can_found {
        return (
            StatusCode::BAD_REQUEST,
            "Insufficient culture points or settlers",
        )
            .into_response();
    }

    let (jar, csrf_token) = generate_csrf(jar);
    let layout_data = create_layout_data(&user, "map");

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data.clone(),
            crate::pages::FoundVillageConfirmationPage {
                village_id: user.village.id,
                village_name: user.village.name.clone(),
                village_position: user.village.position.clone(),
                target_field_id: field_id,
                target_position: field.position.clone(),
                tribe: user.village.tribe.clone(),
                settlers: settlers,
                csrf_token: csrf_token,
            }
        }
    });

    (jar, Html(wrap_in_html(&body_content))).into_response()
}

#[derive(Debug)]
pub struct FoundVillageExecuteForm {
    pub village_id: u32,
    pub target_field_id: u32,
    pub target_x: i32,
    pub target_y: i32,
    pub units: Vec<i32>,
    pub csrf_token: String,
}

impl<'de> Deserialize<'de> for FoundVillageExecuteForm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier)]
        enum Field {
            #[serde(rename = "village_id")]
            VillageId,
            #[serde(rename = "target_field_id")]
            TargetFieldId,
            #[serde(rename = "target_x")]
            TargetX,
            #[serde(rename = "target_y")]
            TargetY,
            #[serde(rename = "units[]")]
            Units,
            #[serde(rename = "csrf_token")]
            CsrfToken,
        }

        struct FormVisitor;

        impl<'de> Visitor<'de> for FormVisitor {
            type Value = FoundVillageExecuteForm;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("found village execute form data")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut village_id = None;
                let mut target_field_id = None;
                let mut target_x = None;
                let mut target_y = None;
                let mut units = Vec::new();
                let mut csrf_token = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::VillageId => {
                            if village_id.is_some() {
                                return Err(de::Error::duplicate_field("village_id"));
                            }
                            village_id = Some(map.next_value()?);
                        }
                        Field::TargetFieldId => {
                            if target_field_id.is_some() {
                                return Err(de::Error::duplicate_field("target_field_id"));
                            }
                            target_field_id = Some(map.next_value()?);
                        }
                        Field::TargetX => {
                            if target_x.is_some() {
                                return Err(de::Error::duplicate_field("target_x"));
                            }
                            target_x = Some(map.next_value()?);
                        }
                        Field::TargetY => {
                            if target_y.is_some() {
                                return Err(de::Error::duplicate_field("target_y"));
                            }
                            target_y = Some(map.next_value()?);
                        }
                        Field::Units => {
                            let value: i32 = map.next_value()?;
                            units.push(value);
                        }
                        Field::CsrfToken => {
                            if csrf_token.is_some() {
                                return Err(de::Error::duplicate_field("csrf_token"));
                            }
                            csrf_token = Some(map.next_value()?);
                        }
                    }
                }

                let village_id =
                    village_id.ok_or_else(|| de::Error::missing_field("village_id"))?;
                let target_field_id =
                    target_field_id.ok_or_else(|| de::Error::missing_field("target_field_id"))?;
                let target_x = target_x.ok_or_else(|| de::Error::missing_field("target_x"))?;
                let target_y = target_y.ok_or_else(|| de::Error::missing_field("target_y"))?;
                let csrf_token =
                    csrf_token.ok_or_else(|| de::Error::missing_field("csrf_token"))?;

                Ok(FoundVillageExecuteForm {
                    village_id,
                    target_field_id,
                    target_x,
                    target_y,
                    units,
                    csrf_token,
                })
            }
        }

        deserializer.deserialize_map(FormVisitor)
    }
}

impl HasCsrfToken for FoundVillageExecuteForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

/// POST /map/field/{id}/found/execute - Execute the founding of a new village
pub async fn found_village_execute(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(_field_id): Path<u32>,
    CsrfForm { jar: _jar, form }: CsrfForm<FoundVillageExecuteForm>,
) -> Response {
    let home_army = match user.village.army() {
        Some(army) => army,
        None => {
            return (StatusCode::BAD_REQUEST, "No army found").into_response();
        }
    };

    let troop_set = parse_troop_set(&form.units);

    let position = Position {
        x: form.target_x,
        y: form.target_y,
    };

    let army_id = home_army.id;

    let command = FoundVillage {
        player_id: user.player.id,
        village_id: user.village.id,
        army_id,
        units: troop_set,
        target_position: position,
    };

    match state
        .app_bus
        .execute(command, FoundVillageCommandHandler::new())
        .await
    {
        Ok(()) => Redirect::to("/build/39").into_response(),
        Err(e) => {
            tracing::error!("Failed to found village: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

fn parse_troop_set(values: &[i32]) -> TroopSet {
    let mut troops = TroopSet::default();
    for idx in 0..troops.units().len() {
        let amount = *values.get(idx).unwrap_or(&0);
        if amount >= 0 {
            troops.set(idx, amount as u32);
        }
    }
    troops
}
