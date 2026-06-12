//! Building workflow and read-model projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::ScheduledActionStatus;
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;
use crate::es::workflows;

impl VillageProjector {
    pub(super) async fn project_building_event_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::BuildingConstructionScheduled { .. }
            | VillageEvent::BuildingUpgradeScheduled { .. }
            | VillageEvent::BuildingDowngradeScheduled { .. } => {
                Some(self.project_building_scheduled(tx, event).await)
            }
            VillageEvent::BuildingConstructionCanceled { .. } => {
                Some(self.project_building_canceled(tx, event).await)
            }
            VillageEvent::BuildingAdded { .. }
            | VillageEvent::BuildingUpgraded { .. }
            | VillageEvent::BuildingDowngraded { .. } => {
                Some(self.project_building_changed(tx, event).await)
            }
            _ => None,
        }
    }

    async fn project_building_scheduled(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let scheduled = workflows::buildings::scheduled_action_from_event(event)?;
        self.add_scheduled_action_in_tx(tx, &scheduled.action)
            .await?;
        if let Some(cost) = &scheduled.cost {
            self.deduct_village_resources_in_tx(tx, scheduled.village_id, cost)
                .await?;
        }
        Ok(())
    }

    async fn project_building_canceled(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let (action_ids, village_id, refund) = match event {
            VillageEvent::BuildingConstructionCanceled {
                action_ids,
                village_id,
                refund,
                ..
            } => (action_ids, village_id, refund),
            _ => unreachable!("project_building_canceled called with non-building cancel event"),
        };

        for action_id in action_ids {
            self.actions
                .update_status_in_tx(tx, *action_id, ScheduledActionStatus::Canceled)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }

        if refund.total() > 0 {
            let village = self
                .village
                .get_by_village_id_in_tx(tx, *village_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            let mut village = Self::village_from_model(&village);
            village.store_resources(refund);
            self.village
                .set_stored_resources_in_tx(tx, *village_id, village.stored_resources())
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }

        Ok(())
    }

    async fn project_building_changed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let (village_id, slot_id, building_name, level, speed) = match event {
            VillageEvent::BuildingAdded {
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                ..
            }
            | VillageEvent::BuildingUpgraded {
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                ..
            }
            | VillageEvent::BuildingDowngraded {
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                ..
            } => (village_id, slot_id, building_name, level, speed),
            _ => unreachable!("project_building_changed called with non-building change event"),
        };
        self.village
            .update_building_in_tx(
                tx,
                *village_id,
                *slot_id,
                building_name.clone(),
                *level,
                *speed,
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(())
    }
}
