use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
};
use dioxus::prelude::*;
use uuid::Uuid;

use parabellum_app::{
    command_handlers::MarkReportReadCommandHandler,
    cqrs::{
        commands::MarkReportRead,
        queries::{GetReportForPlayer, GetReportsForPlayer, ReportView},
    },
    queries_handlers::{GetReportForPlayerHandler, GetReportsForPlayerHandler},
};

use crate::{
    components::{PageLayout, ReportListEntry, wrap_in_html},
    handlers::helpers::{CurrentUser, create_layout_data},
    http::AppState,
    pages::{BattleReportPage, ReinforcementReportPage, ReportsPage},
    view_helpers::format_resource_summary,
};
use parabellum_types::{battle::AttackType, reports::ReportPayload};
use rust_i18n::t;

/// GET /reports
pub async fn reports_page(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
    let raw_reports = state
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

    let report_entries: Vec<ReportListEntry> = raw_reports.into_iter().map(map_report).collect();
    let layout_data = create_layout_data(&user, "reports");
    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            ReportsPage { reports: report_entries }
        }
    });

    Html(wrap_in_html(&body_content))
}

/// GET /reports/{id}
pub async fn report_page(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(report_id): Path<Uuid>,
) -> impl IntoResponse {
    let player_id = user.player.id;
    let report = match state
        .app_bus
        .query(
            GetReportForPlayer {
                report_id,
                player_id,
            },
            GetReportForPlayerHandler::new(),
        )
        .await
    {
        Ok(Some(report)) => report,
        Ok(None) | Err(_) => return Redirect::to("/reports").into_response(),
    };

    let layout_data = create_layout_data(&user, "reports");
    let _ = state
        .app_bus
        .execute(
            MarkReportRead {
                report_id,
                player_id,
            },
            MarkReportReadCommandHandler::new(),
        )
        .await;

    render_report_page(report, layout_data)
}

fn map_report(report: ReportView) -> ReportListEntry {
    let (title, summary) = match report.payload.clone() {
        parabellum_types::reports::ReportPayload::Battle(payload) => {
            // Verb based on attack type
            let verb = match (payload.attack_type, payload.scouting.is_some()) {
                (_, true) => "scouted",
                (AttackType::Raid, _) => "raided",
                (AttackType::Normal, _) => "attacked",
            };

            // Title: "VillageA attacked VillageB"
            let title = format!(
                "{} {} {}",
                payload.attacker_village, verb, payload.defender_village
            );

            // Result and outcome
            let result = if payload.success {
                t!("game.reports.battle_success")
            } else {
                t!("game.reports.battle_failure")
            };

            let outcome = if payload.bounty.total() > 0 {
                format!("Bounty: {}", format_resource_summary(&payload.bounty))
            } else if let Some(ref attacker) = payload.attacker {
                let total_losses: u32 = attacker.losses.iter().sum();
                if total_losses > 0 {
                    format!("Lost {} units", total_losses)
                } else {
                    "No losses".to_string()
                }
            } else {
                "".to_string()
            };

            // Summary with positions: "VillageA (X|Y) attacked VillageB (Z|W) - Victory - Bounty: 500"
            let summary = format!(
                "{} ({}|{}) {} {} ({}|{}) - {} - {}",
                payload.attacker_village,
                payload.attacker_position.x,
                payload.attacker_position.y,
                verb,
                payload.defender_village,
                payload.defender_position.x,
                payload.defender_position.y,
                result,
                outcome
            );

            (title, summary)
        }
        parabellum_types::reports::ReportPayload::Reinforcement(payload) => {
            // Title: "VillageA reinforced VillageB"
            let title = format!(
                "{} reinforced {}",
                payload.sender_village, payload.receiver_village
            );

            // Summary with positions and troop count
            let total_troops: u32 = payload.units.iter().sum();
            let summary = format!(
                "{} ({}|{}) reinforced {} ({}|{}) - {} troops sent",
                payload.sender_village,
                payload.sender_position.x,
                payload.sender_position.y,
                payload.receiver_village,
                payload.receiver_position.x,
                payload.receiver_position.y,
                total_troops
            );

            (title, summary)
        }
    };

    ReportListEntry {
        id: report.id,
        title,
        summary,
        created_at: report.created_at,
        created_at_formatted: report.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        is_read: report.read_at.is_some(),
        permalink: format!("/reports/{}", report.id),
    }
}

fn render_report_page(report: ReportView, layout_data: crate::components::LayoutData) -> Response {
    match report.payload {
        ReportPayload::Battle(payload) => {
            let body_content = dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data,
                    BattleReportPage {
                        report_id: report.id,
                        created_at: report.created_at,
                        payload: payload
                    }
                }
            });
            Html(wrap_in_html(&body_content)).into_response()
        }
        ReportPayload::Reinforcement(payload) => {
            let body_content = dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data,
                    ReinforcementReportPage {
                        report_id: report.id,
                        created_at: report.created_at,
                        payload: payload
                    }
                }
            });
            Html(wrap_in_html(&body_content)).into_response()
        }
    }
}
