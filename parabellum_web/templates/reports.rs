use askama::Template;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::shared::TemplateLayout;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ReportListEntry {
    pub id: Uuid,
    pub title: String,
    pub summary: String,
    pub created_at: DateTime<Utc>,
    pub created_at_formatted: String,
    pub is_read: bool,
    pub permalink: String,
}

#[derive(Debug, Template)]
#[allow(dead_code)]
#[template(path = "reports/index.html")]
pub struct ReportsTemplate {
    pub layout: TemplateLayout,
    pub reports: Vec<ReportListEntry>,
}

#[derive(Debug, Template)]
#[allow(dead_code)]
#[template(path = "reports/battle.html")]
pub struct BattleReportTemplate {
    pub layout: TemplateLayout,
    pub report_reference: String,
    pub report_reference_label: String,
    pub created_at_formatted: String,
    pub attacker_player: String,
    pub attacker_village: String,
    pub defender_player: String,
    pub defender_village: String,
    pub result_label: String,
    pub success: bool,
    pub bounty_summary: String,
}

#[derive(Debug, Template)]
#[allow(dead_code)]
#[template(path = "reports/generic.html")]
pub struct GenericReportTemplate {
    pub layout: TemplateLayout,
    pub report_reference: String,
    pub report_reference_label: String,
    pub created_at_formatted: String,
    pub heading: String,
    pub message: String,
}
