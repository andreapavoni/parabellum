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
}

#[derive(Debug, Template)]
#[allow(dead_code)]
#[template(path = "reports/index.html")]
pub struct ReportsTemplate {
    pub layout: TemplateLayout,
    pub reports: Vec<ReportListEntry>,
}
