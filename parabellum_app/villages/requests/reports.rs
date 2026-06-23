//! Report use-case request types.
//!
//! These DTOs represent application inputs for projected report reads and
//! report state changes.

use uuid::Uuid;

/// Request to list reports visible to one player.
#[derive(Debug, Clone, Copy)]
pub struct ListReportsForPlayerRequest {
    /// Player whose report inbox is being listed.
    pub player_id: Uuid,
    /// Zero-based result offset.
    pub offset: i64,
    /// Maximum number of reports to return.
    pub limit: i64,
}

/// Request to load one report visible to one player.
#[derive(Debug, Clone, Copy)]
pub struct GetReportForPlayerRequest {
    /// Report id to load.
    pub report_id: Uuid,
    /// Player that must be allowed to see the report.
    pub player_id: Uuid,
}

/// Request to count unread reports for one player.
#[derive(Debug, Clone, Copy)]
pub struct CountUnreadReportsForPlayerRequest {
    /// Player whose unread report count is requested.
    pub player_id: Uuid,
}

/// Request to mark one report as read for one player.
#[derive(Debug, Clone, Copy)]
pub struct MarkReportReadRequest {
    /// Report id to mark as read.
    pub report_id: Uuid,
    /// Player that must be allowed to read the report.
    pub player_id: Uuid,
}
