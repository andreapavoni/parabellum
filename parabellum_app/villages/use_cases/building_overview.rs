//! Common building query orchestration; specialized feature data is loaded separately.
use crate::villages::{
    BuildingSettings, VillageArmyContext, hydrate_village_at,
    models::BuildingWorkflowKind,
    policies::{
        building_preview::{empty_preview, occupied_preview},
        building_queue::building_queue_capacity,
    },
    ports::{Clock, VillageActivityReadPort, VillageStateReadPort},
    read_models::buildings::{BuildingOverview, BuildingSlotOverview},
    requests::building_overview::GetBuildingOverviewRequest,
};
use parabellum_types::errors::{ApplicationError, GameError};
use std::sync::Arc;

/// Reads one village and its queues, validates ownership, and computes common previews.
#[derive(Clone)]
pub struct BuildingOverviewUseCases {
    villages: Arc<dyn VillageStateReadPort>,
    activity: Arc<dyn VillageActivityReadPort>,
    clock: Arc<dyn Clock>,
    settings: BuildingSettings,
}

impl BuildingOverviewUseCases {
    /// Uses existing read capabilities without introducing feature-specific repository work.
    pub fn new(
        villages: Arc<dyn VillageStateReadPort>,
        activity: Arc<dyn VillageActivityReadPort>,
        clock: Arc<dyn Clock>,
        settings: BuildingSettings,
    ) -> Self {
        Self {
            villages,
            activity,
            clock,
            settings,
        }
    }

    /// Returns structural previews and authoritative projected stocks for an owned slot.
    pub async fn get_building_overview(
        &self,
        request: GetBuildingOverviewRequest,
    ) -> Result<BuildingOverview, ApplicationError> {
        if !(1..=40).contains(&request.slot_id) {
            return Err(GameError::EmptySlot {
                slot_id: request.slot_id,
            }
            .into());
        }
        let village = self.villages.get_village_state(request.village_id).await?;
        if village.village_id != request.village_id || village.player_id != request.player_id {
            return Err(GameError::VillageNotOwned {
                village_id: request.village_id,
                player_id: request.player_id,
            }
            .into());
        }
        let queues = self.activity.get_village_queues(request.village_id).await?;
        let server_time = self.clock.now();
        // The read port already refreshes economy with army/hero context. Only use
        // this domain value for structural building rules; do not tick stocks a
        // second time without that context.
        let domain = hydrate_village_at(
            village.clone(),
            VillageArmyContext::default(),
            village.updated_at,
        );
        let mut queue_full = queues.building.len() >= building_queue_capacity(&village.tribe);
        let slot = if let Some(building) = domain.get_building_by_slot_id(request.slot_id) {
            queue_full |= queues.building.iter().any(|item| {
                item.slot_id == request.slot_id
                    && matches!(item.kind, BuildingWorkflowKind::Downgrade)
            });
            let upgrade = occupied_preview(
                &domain,
                &building,
                &queues.building,
                self.settings.server_speed,
            );
            BuildingSlotOverview::Occupied { building, upgrade }
        } else {
            BuildingSlotOverview::Empty(empty_preview(
                &domain,
                request.slot_id,
                &queues.building,
                self.settings.server_speed,
            ))
        };
        Ok(BuildingOverview {
            server_time,
            village,
            queues,
            queue_full,
            slot,
        })
    }
}

#[cfg(test)]
mod tests;
