use crate::{
    components::{PageLayout, wrap_in_html},
    handlers::helpers::{CurrentUser, create_layout_data},
    http::AppState,
    pages::{
        StatsPage,
        stats::{LeaderboardEntry, PaginationInfo},
    },
};
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
};
use dioxus::prelude::*;
use parabellum_app::{cqrs::queries::GetLeaderboard, queries_handlers::GetLeaderboardHandler};
use serde::Deserialize;

const LEADERBOARD_PAGE_SIZE: i64 = 20;

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub page: Option<i64>,
}

/// GET /stats - Render the leaderboard page
pub async fn stats_page(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(params): Query<StatsQuery>,
) -> impl IntoResponse {
    let requested_page = params.page.unwrap_or(1).max(1);

    let data = state
        .app_bus
        .query(
            GetLeaderboard {
                page: requested_page,
                per_page: LEADERBOARD_PAGE_SIZE,
            },
            GetLeaderboardHandler::new(),
        )
        .await;

    let mut entries;
    let total_players;
    match data {
        Ok(result) => {
            entries = result.entries;
            total_players = result.total_players;
        }
        Err(e) => {
            tracing::error!("Unable to load leaderboard: {}", e);
            entries = Vec::new();
            total_players = 0;
        }
    }

    let total_pages = if total_players == 0 {
        1
    } else {
        (total_players + LEADERBOARD_PAGE_SIZE - 1) / LEADERBOARD_PAGE_SIZE
    };

    let mut page = requested_page;

    if total_players > 0 && page > total_pages {
        page = total_pages;
        // Try to load the last available page to avoid showing an empty list on overflow.
        if let Ok(fallback) = state
            .app_bus
            .query(
                GetLeaderboard {
                    page,
                    per_page: LEADERBOARD_PAGE_SIZE,
                },
                GetLeaderboardHandler::new(),
            )
            .await
        {
            entries = fallback.entries;
        }
    }

    let rows: Vec<LeaderboardEntry> = entries
        .into_iter()
        .enumerate()
        .map(|(idx, entry)| LeaderboardEntry {
            player_id: entry.player_id.to_string(),
            rank: (page - 1) * LEADERBOARD_PAGE_SIZE + idx as i64 + 1,
            username: entry.username,
            tribe: format!("{:?}", entry.tribe),
            village_count: entry.village_count,
            population: entry.population,
        })
        .collect();

    let pagination = PaginationInfo {
        page,
        per_page: LEADERBOARD_PAGE_SIZE,
        total_players,
        total_pages: total_pages.max(1),
    };

    let layout_data = create_layout_data(&user, "stats");

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data.clone(),
            StatsPage {
                entries: rows,
                pagination
            }
        }
    });

    Html(wrap_in_html(&body_content))
}
