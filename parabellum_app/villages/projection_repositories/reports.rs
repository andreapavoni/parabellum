//! Report projection repository contracts.

use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::models::ReportModel;

/// High-level report category used by application read filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    /// Battle, raid, scout, conquest, trap, and related combat reports.
    Battle,
    /// Marketplace delivery reports.
    MarketplaceDelivery,
    /// Reinforcement arrival and support reports.
    Reinforcement,
}

impl ReportKind {
    /// Returns the canonical read-model discriminator stored in `rm_reports.report_type`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Battle => "battle",
            Self::MarketplaceDelivery => "marketplace_delivery",
            Self::Reinforcement => "reinforcement",
        }
    }
}

/// Audience-scoped report query filter.
///
/// Reports are only visible through `rm_report_reads`, so every filter is
/// anchored to one player audience. Additional predicates narrow the visible
/// reports without exposing SQL details to application callers.
#[derive(Debug, Clone)]
pub struct ReportFilter {
    /// Player audience that must be allowed to see the report rows.
    pub player_id: Uuid,
    /// Optional single report id restriction.
    pub report_id: Option<Uuid>,
    /// Optional report category restrictions.
    pub kinds: Vec<ReportKind>,
    /// When true, only audience rows with no `read_at` timestamp are returned.
    pub unread_only: bool,
    /// Optional list offset.
    pub offset: Option<i64>,
    /// Optional list limit.
    pub limit: Option<i64>,
}

impl ReportFilter {
    /// Creates a filter for reports visible to `player_id`.
    pub fn for_player(player_id: Uuid) -> Self {
        Self {
            player_id,
            report_id: None,
            kinds: Vec::new(),
            unread_only: false,
            offset: None,
            limit: None,
        }
    }

    /// Restricts the filter to a single report id.
    pub fn report(mut self, report_id: Uuid) -> Self {
        self.report_id = Some(report_id);
        self
    }

    /// Restricts the filter to unread reports.
    pub fn unread(mut self) -> Self {
        self.unread_only = true;
        self
    }

    /// Restricts the filter to one report category.
    pub fn kind(mut self, kind: ReportKind) -> Self {
        self.kinds = vec![kind];
        self
    }

    /// Restricts the filter to any of the given report categories.
    pub fn kinds(mut self, kinds: impl IntoIterator<Item = ReportKind>) -> Self {
        self.kinds = kinds.into_iter().collect();
        self
    }

    /// Applies offset/limit pagination.
    pub fn page(mut self, offset: i64, limit: i64) -> Self {
        self.offset = Some(offset);
        self.limit = Some(limit);
        self
    }
}

/// Report projection row before audience materialization.
#[derive(Debug, Clone)]
pub struct ProjectedReport {
    pub id: Uuid,
    pub report_type: String,
    pub payload: serde_json::Value,
    pub actor_player_id: Uuid,
    pub actor_village_id: Option<u32>,
    pub target_player_id: Option<Uuid>,
    pub target_village_id: Option<u32>,
}

/// Persistence boundary for projected reports and report audiences.
#[async_trait::async_trait]
pub trait ReportRepository: Send + Sync {
    /// Stores a projected report and materializes its player audience rows.
    async fn add_projected(
        &self,
        report: &ProjectedReport,
        audience_player_ids: &[Uuid],
    ) -> Result<(), ApplicationError>;

    /// Lists reports matching an audience-scoped filter.
    async fn list_reports(
        &self,
        filter: ReportFilter,
    ) -> Result<Vec<ReportModel>, ApplicationError>;

    /// Loads the first report matching an audience-scoped filter.
    async fn find_report(
        &self,
        filter: ReportFilter,
    ) -> Result<Option<ReportModel>, ApplicationError>;

    /// Counts reports matching an audience-scoped filter.
    async fn count_reports(&self, filter: ReportFilter) -> Result<i64, ApplicationError>;

    /// Marks one audience row as read.
    async fn mark_as_read(&self, report_id: Uuid, player_id: Uuid) -> Result<(), ApplicationError>;
}
