//! Read/context port for building lifecycle use cases.
//!
//! Building cancellation needs current scheduled-action context before command
//! intent can be built.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parabellum_types::{common::ResourceGroup, errors::ApplicationError};
use uuid::Uuid;

/// Current workflow context for a queued building construction cancellation.
#[derive(Debug, Clone)]
pub struct CancelBuildingConstructionContext {
    /// Scheduled action ids canceled together with the selected action.
    pub action_ids: Vec<Uuid>,
    /// Player that owns the queued construction.
    pub player_id: Uuid,
    /// Village that owns the queued construction.
    pub village_id: u32,
    /// Time when the selected construction would execute.
    pub execute_at: DateTime<Utc>,
    /// Resources refunded by cancellation.
    pub refund: ResourceGroup,
}

/// Loads read-model context required by building lifecycle use cases.
#[async_trait]
pub trait BuildingReadPort: Send + Sync {
    /// Returns cancelable building construction context for a scheduled action.
    async fn get_cancel_building_construction_context(
        &self,
        village_id: u32,
        action_id: Uuid,
        canceled_at: DateTime<Utc>,
    ) -> Result<CancelBuildingConstructionContext, ApplicationError>;
}
