//! Report command execution gateway.
//!
//! The app layer turns report state changes into explicit command intent. The
//! infrastructure layer decides how that intent is persisted.

use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use crate::villages::MarkReportRead;

/// Canonical report command intent produced by app use cases.
#[derive(Debug, Clone)]
pub enum ReportCommandIntent {
    /// Mark a projected report as read through its village stream anchor.
    MarkReportRead {
        /// Village aggregate id used to append the report event.
        village_id: u32,
        /// Domain command with report ownership and timestamp data.
        command: MarkReportRead,
    },
}

/// Executes report command intent through infrastructure.
#[async_trait]
pub trait ReportCommandExecutor: Send + Sync {
    /// Persist and execute an already-planned report command intent.
    async fn execute_report_command(
        &self,
        command: ReportCommandIntent,
    ) -> Result<(), ApplicationError>;
}
