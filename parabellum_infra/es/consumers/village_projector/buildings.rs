//! Building workflow and read-model projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::models::ScheduledActionStatus;
use parabellum_app::villages::{VillageEvent, apply_domain_village_state};
use parabellum_game::models::buildings::Building;
use parabellum_game::models::village::Village;
use parabellum_types::buildings::BuildingName;
use sqlx::{Postgres, Transaction};

use super::economy::VillageEconomyFacts;
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
        let scheduled = workflows::buildings::building_scheduled_action_from_event(event)?;
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
                .map_err(CqrsError::domain_source)?;
        }

        if refund.total() > 0 {
            let village = self
                .village
                .get_by_village_id_in_tx(tx, *village_id)
                .await
                .map_err(CqrsError::domain_source)?;
            let mut village = self.load_village_state_in_tx(tx, village).await?;
            village.store_resources_fact(refund);
            self.apply_village_economy_facts_in_tx(
                tx,
                *village_id,
                VillageEconomyFacts::stored_resources(village.stored_resources()),
            )
            .await?;
        }

        Ok(())
    }

    async fn project_building_changed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let village_id = match event {
            VillageEvent::BuildingAdded { village_id, .. }
            | VillageEvent::BuildingUpgraded { village_id, .. }
            | VillageEvent::BuildingDowngraded { village_id, .. } => *village_id,
            _ => unreachable!("project_building_changed called with non-building change event"),
        };

        let mut model = self
            .village
            .get_by_village_id_in_tx(tx, village_id)
            .await
            .map_err(CqrsError::domain_source)?;
        let mut village = self.load_village_state_in_tx(tx, model.clone()).await?;

        let (building_name, slot_id, level, speed) = match event {
            VillageEvent::BuildingAdded {
                slot_id,
                building_name,
                level,
                speed,
                ..
            }
            | VillageEvent::BuildingUpgraded {
                slot_id,
                building_name,
                level,
                speed,
                ..
            }
            | VillageEvent::BuildingDowngraded {
                slot_id,
                building_name,
                level,
                speed,
                ..
            } => (building_name, slot_id, level, speed),
            _ => unreachable!("project_building_changed called with non-building change event"),
        };
        apply_building_level(
            &mut village,
            building_name.clone(),
            *slot_id,
            *level,
            *speed,
        )?;

        apply_domain_village_state(&mut model, &village);
        self.village
            .store_village_model_in_tx(tx, &model)
            .await
            .map_err(CqrsError::domain_source)?;
        Ok(())
    }
}

fn apply_building_level(
    village: &mut Village,
    building_name: BuildingName,
    slot_id: u8,
    level: u8,
    speed: i8,
) -> Result<(), CqrsError> {
    let building = if level == 0 && slot_id > 18 {
        None
    } else {
        Some(
            Building::new(building_name, speed)
                .at_level(level, speed)
                .map_err(CqrsError::domain_source)?,
        )
    };
    village.set_building_fact(slot_id, building);
    Ok(())
}
