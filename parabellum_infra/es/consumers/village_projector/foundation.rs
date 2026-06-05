//! Settler movement read-model projection for village foundation.
//!
//! Foundation workflow decisions live in `es::workflows::foundation`; this
//! module only projects sent/arrived facts into movement, army, resource, and
//! scheduled-action read models.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{MovementDirection, MovementType, VillageMovement};
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;
use crate::es::workflows;

impl VillageProjector {
    pub(super) async fn project_foundation_event_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::SettlersSent { .. } => Some(self.project_settlers_sent(tx, event).await),
            VillageEvent::SettlersArrived { .. } => {
                Some(self.project_settlers_arrived(tx, event).await)
            }
            _ => None,
        }
    }

    async fn project_settlers_sent(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::SettlersSent {
            movement_id,
            player_id,
            source_village_id,
            target_position,
            village_name,
            tribe,
            army,
            arrives_at,
            ..
        } = event
        else {
            unreachable!("project_settlers_sent called with non-SettlersSent event");
        };

        self.armies
            .upsert_moving_in_tx(tx, army, *source_village_id, *player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        if let Some(hero) = army.hero() {
            self.heroes
                .upsert_in_tx(tx, &hero, hero.village_id, *source_village_id, "moving")
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }

        let outgoing = VillageMovement {
            movement_id: *movement_id,
            movement_type: MovementType::FoundVillage,
            direction: MovementDirection::Outgoing,
            origin_village_id: *source_village_id,
            origin_village_name: None,
            origin_player_id: *player_id,
            origin_position: None,
            // `rm_village_movements.target_village_id` is FK-backed and an
            // unoccupied valley has no rm_village row yet. Keep FK valid
            // and carry real destination in `target_position`.
            target_village_id: *source_village_id,
            target_village_name: Some(village_name.clone()),
            target_player_id: Some(*player_id),
            target_position: Some(target_position.clone()),
            arrives_at: *arrives_at,
            time_seconds: None,
            units: army.units().clone(),
            tribe: Some(tribe.clone()),
            bounty: None,
        };
        self.movements
            .upsert_in_tx(tx, &outgoing)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let source = self
            .village
            .get_by_village_id_in_tx(tx, *source_village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let mut source = Self::village_from_model(&source);
        source
            .deduct_foundation_resources()
            .map_err(CqrsError::domain_source)?;
        self.set_stored_resources_in_tx(tx, *source_village_id, source.stored_resources())
            .await?;

        let action = workflows::movements::settlers_arrival_scheduled_action_from_event(event)?;
        self.add_scheduled_action_in_tx(tx, &action).await
    }

    async fn project_settlers_arrived(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::SettlersArrived {
            movement_id,
            army_id,
            ..
        } = event
        else {
            unreachable!("project_settlers_arrived called with non-SettlersArrived event");
        };
        self.movements
            .delete_by_movement_id_in_tx(tx, *movement_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.armies
            .delete_in_tx(tx, *army_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }
}
