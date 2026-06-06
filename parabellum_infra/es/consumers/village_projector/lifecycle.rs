//! Village lifecycle and simple village-state projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;

impl VillageProjector {
    pub(super) async fn project_lifecycle_event_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
        aggregate_id: &str,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::VillageFounded { .. } => {
                Some(self.project_village_founded(tx, event).await)
            }
            VillageEvent::VillageConquered { .. } => Some(
                self.project_village_conquered(tx, event, aggregate_id)
                    .await,
            ),
            VillageEvent::VillageResourcesSet { .. } => {
                Some(self.project_village_resources_set(tx, event).await)
            }
            VillageEvent::VillageRenamed { .. } => {
                Some(self.project_village_renamed(tx, event).await)
            }
            _ => None,
        }
    }

    async fn project_village_founded(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::VillageFounded {
            village_id,
            village_name,
            position,
            tribe,
            player_id,
            parent_village_id,
            buildings,
            ..
        } = event
        else {
            unreachable!("project_village_founded called with non-VillageFounded event");
        };
        self.village
            .upsert_from_village_in_tx(
                tx,
                *village_id,
                *player_id,
                village_name,
                position,
                tribe.clone(),
                *parent_village_id,
                buildings,
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.village
            .set_map_occupancy_in_tx(tx, *village_id, Some(*village_id), Some(*player_id))
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn project_village_conquered(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
        aggregate_id: &str,
    ) -> Result<(), CqrsError> {
        let VillageEvent::VillageConquered {
            player_id,
            owner_village_id,
        } = event
        else {
            unreachable!("project_village_conquered called with non-VillageConquered event");
        };
        let village_id = aggregate_id
            .parse::<u32>()
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let mut conquered = self
            .village
            .get_by_village_id_in_tx(tx, village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        conquered.player_id = *player_id;
        conquered.parent_village_id = Some(*owner_village_id);
        conquered.loyalty = 0;
        self.village
            .replace_village_state_in_tx(tx, &conquered)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.village
            .set_map_occupancy_in_tx(tx, village_id, Some(village_id), Some(*player_id))
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn project_village_resources_set(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::VillageResourcesSet {
            village_id,
            resources,
            ..
        } = event
        else {
            unreachable!("project_village_resources_set called with non-VillageResourcesSet event");
        };
        self.set_stored_resources_in_tx(tx, *village_id, resources.clone())
            .await
    }

    async fn project_village_renamed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::VillageRenamed {
            village_id,
            village_name,
            ..
        } = event
        else {
            unreachable!("project_village_renamed called with non-VillageRenamed event");
        };
        let mut village = self
            .village
            .get_by_village_id_in_tx(tx, *village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        village.village_name = village_name.clone();
        self.village
            .replace_village_state_in_tx(tx, &village)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }
}
