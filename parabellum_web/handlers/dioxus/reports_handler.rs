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
    components::{
        BattleReportPage, GenericReportData, GenericReportPage, PageLayout, ReportListEntry,
        ReportsPage, wrap_in_html,
    },
    handlers::{CurrentUser, dioxus::helpers::create_layout_data},
    http::AppState,
    view_helpers::format_resource_summary,
};
use parabellum_types::reports::ReportPayload;
use rust_i18n::t;

pub async fn reports(State(state): State<AppState>, user: CurrentUser) -> impl IntoResponse {
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

pub async fn report_detail(
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
            let title = format!(
                "{} → {}",
                payload.attacker_village, payload.defender_village
            );
            let result = if payload.success {
                t!("game.reports.battle_success")
            } else {
                t!("game.reports.battle_failure")
            };
            let bounty = format_resource_summary(&payload.bounty);
            let summary = t!(
                "game.reports.battle_summary",
                attacker = payload.attacker_player,
                defender = payload.defender_player,
                result = result,
                bounty = bounty
            )
            .into_owned();
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
        _ => {
            // Generic report fallback
            let created_at_formatted = report.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
            let report_reference = report.id.to_string();
            let report_reference_label =
                t!("game.reports.detail_id", id = report_reference.clone()).into_owned();

            let data = GenericReportData {
                report_reference,
                report_reference_label,
                created_at_formatted,
                heading: t!("game.reports.generic_title").to_string(),
                message: t!("game.reports.generic_message").to_string(),
            };

            let body_content = dioxus_ssr::render_element(rsx! {
                PageLayout {
                    data: layout_data,
                    GenericReportPage { data: data }
                }
            });
            Html(wrap_in_html(&body_content)).into_response()
        }
    }
}
