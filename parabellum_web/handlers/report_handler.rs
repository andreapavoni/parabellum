use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
};
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
    handlers::{CurrentUser, render_template},
    http::AppState,
    templates::{
        BattleReportTemplate, GenericReportTemplate, ReportListEntry, ReportsTemplate,
        TemplateLayout,
    },
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

    let report_entries: Vec<ReportListEntry> = raw_reports.into_iter().map(map_report).collect();

    let template = ReportsTemplate {
        layout: TemplateLayout::new(Some(user), "reports"),
        reports: report_entries,
    };

    render_template(template, None)
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

    let layout = TemplateLayout::new(Some(user), "reports");
    let page = report_page(report, layout);

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

    render_report_page(page)
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

enum ReportDetailPage {
    Battle(BattleReportTemplate),
    Generic(GenericReportTemplate),
}

fn render_report_page(page: ReportDetailPage) -> Response {
    match page {
        ReportDetailPage::Battle(template) => render_template(template, None).into_response(),
        ReportDetailPage::Generic(template) => render_template(template, None).into_response(),
    }
}

#[allow(unreachable_patterns)]
fn report_page(report: ReportView, layout: TemplateLayout) -> ReportDetailPage {
    let created_at_formatted = report.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
    let report_reference = report.id.to_string();
    let report_reference_label =
        t!("game.reports.detail_id", id = report_reference.clone()).into_owned();
    match report.payload {
        parabellum_types::reports::ReportPayload::Battle(payload) => {
            let result_label = if payload.success {
                t!("game.reports.battle_success")
            } else {
                t!("game.reports.battle_failure")
            }
            .into_owned();
            ReportDetailPage::Battle(BattleReportTemplate {
                layout,
                report_reference: report_reference.clone(),
                report_reference_label: report_reference_label.clone(),
                created_at_formatted,
                attacker_player: payload.attacker_player,
                attacker_village: payload.attacker_village,
                defender_player: payload.defender_player,
                defender_village: payload.defender_village,
                result_label,
                success: payload.success,
                bounty_summary: format_resource_summary(&payload.bounty),
            })
        }
        _ => ReportDetailPage::Generic(GenericReportTemplate {
            layout,
            report_reference,
            report_reference_label,
            created_at_formatted,
            heading: t!("game.reports.generic_title").to_string(),
            message: t!("game.reports.generic_message").to_string(),
        }),
    }
}
