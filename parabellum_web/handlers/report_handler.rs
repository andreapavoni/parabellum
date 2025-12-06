use axum::{extract::State, response::IntoResponse};

use parabellum_app::{
    command_handlers::MarkReportReadCommandHandler,
    cqrs::{
        commands::MarkReportRead,
        queries::{GetReportsForPlayer, ReportView},
    },
    queries_handlers::GetReportsForPlayerHandler,
};

use crate::{
    handlers::{CurrentUser, render_template},
    http::AppState,
    templates::{ReportListEntry, ReportsTemplate, TemplateLayout},
    view_helpers::format_resource_summary,
};
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

    let mut unread_ids = Vec::new();
    let report_entries: Vec<ReportListEntry> = raw_reports
        .into_iter()
        .map(|report| {
            if report.read_at.is_none() {
                unread_ids.push(report.id);
            }
            map_report(report)
        })
        .collect();

    for id in unread_ids {
        let _ = state
            .app_bus
            .execute(
                MarkReportRead {
                    report_id: id,
                    player_id: user.player.id,
                },
                MarkReportReadCommandHandler::new(),
            )
            .await;
    }

    let template = ReportsTemplate {
        layout: TemplateLayout::new(Some(user), "reports"),
        reports: report_entries,
    };

    render_template(template, None)
}

fn map_report(report: ReportView) -> ReportListEntry {
    let (title, summary) = match report.payload {
        parabellum_types::reports::ReportPayload::Battle(payload) => {
            let title = format!(
                "{} → {}",
                payload.attacker_village, payload.defender_village
            );
            let result = if payload.success {
                t!("reports.battle_success")
            } else {
                t!("reports.battle_failure")
            };
            let bounty = format_resource_summary(&payload.bounty);
            let summary = t!(
                "reports.battle_summary",
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
    }
}
