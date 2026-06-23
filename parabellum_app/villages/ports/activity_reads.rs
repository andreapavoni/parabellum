//! Read/context port for village activity views.
//!
//! Activity reads are app-facing queue and movement summaries used by the UI.

use std::collections::HashSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::read_models::{VillageQueues, VillageTroopMovements};

/// Loads app-facing village activity views.
#[async_trait]
pub trait VillageActivityReadPort: Send + Sync {
    /// Returns current building, training, research, and trap queues.
    async fn get_village_queues(&self, village_id: u32) -> Result<VillageQueues, ApplicationError>;

    /// Returns current incoming and outgoing troop movements.
    async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<VillageTroopMovements, ApplicationError>;

    /// Lists outgoing troop movement ids that can still be canceled.
    async fn list_cancelable_outgoing_movement_ids(
        &self,
        village_id: u32,
        now: DateTime<Utc>,
    ) -> Result<HashSet<Uuid>, ApplicationError>;
}
