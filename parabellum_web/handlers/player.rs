use crate::{
    components::{PageLayout, wrap_in_html},
    handlers::helpers::{CurrentUser, create_layout_data},
    http::AppState,
    pages::{PlayerProfilePage, player::PlayerVillageRow},
};
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
};
use dioxus::prelude::*;
use parabellum_app::{
    cqrs::queries::{GetPlayerById, ListVillagesByPlayerId},
    queries_handlers::{GetPlayerByIdHandler, ListVillagesByPlayerIdHandler},
};
use uuid::Uuid;

/// GET /players/{id} - Player profile with village list
pub async fn player_profile(
    State(state): State<AppState>,
    Path(player_id): Path<Uuid>,
    user: CurrentUser,
) -> Response {
    let player = match state
        .app_bus
        .query(GetPlayerById { player_id }, GetPlayerByIdHandler::new())
        .await
    {
        Ok(player) => player,
        Err(e) => {
            tracing::warn!("Player {player_id} not found: {}", e);
            return Redirect::to("/stats").into_response();
        }
    };

    let villages = match state
        .app_bus
        .query(
            ListVillagesByPlayerId { player_id },
            ListVillagesByPlayerIdHandler::new(),
        )
        .await
    {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Error loading villages for player {player_id}: {}", e);
            Vec::new()
        }
    };

    let rows: Vec<PlayerVillageRow> = villages
        .into_iter()
        .map(|v| PlayerVillageRow {
            village_id: v.id,
            name: v.name,
            x: v.position.x,
            y: v.position.y,
            population: v.population as i32,
        })
        .collect();

    let layout_data = create_layout_data(&user, "stats");

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data.clone(),
            PlayerProfilePage {
                username: player.username,
                villages: rows
            }
        }
    });

    Html(wrap_in_html(&body_content)).into_response()
}
