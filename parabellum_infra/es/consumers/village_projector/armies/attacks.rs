//! Attack and scout movement projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::MovementType;
use parabellum_types::battle::AttackType;
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;
use crate::es::consumers::village_projector::armies::MovementPairProjection;
use crate::es::workflows;

impl VillageProjector {
    pub(super) async fn project_attack_sent(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::AttackSent {
            movement_id,
            player_id,
            source_village_id,
            target_village_id,
            army,
            attack_type,
            arrives_at,
            ..
        } = event
        else {
            unreachable!("project_attack_sent called with non-AttackSent event");
        };
        self.upsert_moving_army(tx, army, *source_village_id, *player_id)
            .await?;
        let movement_type = match attack_type {
            AttackType::Raid => MovementType::Raid,
            AttackType::Normal => MovementType::Attack,
        };
        self.upsert_movement_pair(
            tx,
            MovementPairProjection {
                movement_id: *movement_id,
                movement_type,
                player_id: *player_id,
                source_village_id: *source_village_id,
                target_village_id: *target_village_id,
                arrives_at: *arrives_at,
                army,
            },
        )
        .await
    }

    pub(super) async fn project_scout_sent(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::ScoutSent {
            movement_id,
            player_id,
            source_village_id,
            target_village_id,
            army,
            arrives_at,
            ..
        } = event
        else {
            unreachable!("project_scout_sent called with non-ScoutSent event");
        };
        self.upsert_moving_army(tx, army, *source_village_id, *player_id)
            .await?;
        self.upsert_movement_pair(
            tx,
            MovementPairProjection {
                movement_id: *movement_id,
                movement_type: MovementType::Scout,
                player_id: *player_id,
                source_village_id: *source_village_id,
                target_village_id: *target_village_id,
                arrives_at: *arrives_at,
                army,
            },
        )
        .await?;

        let action = workflows::movements::scout_arrival_scheduled_action_from_event(event)?;
        self.add_scheduled_action_in_tx(tx, &action).await
    }

    pub(super) async fn project_attack_arrival_scheduled(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::AttackArrivalScheduled { .. } = event else {
            unreachable!(
                "project_attack_arrival_scheduled called with non-AttackArrivalScheduled event"
            );
        };
        let action = workflows::movements::attack_arrival_scheduled_action_from_event(event)?;
        self.add_scheduled_action_in_tx(tx, &action).await
    }

    pub(super) async fn project_army_arrived(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let (movement_id, army_id) = match event {
            VillageEvent::AttackArrived {
                movement_id,
                army_id,
                ..
            }
            | VillageEvent::ScoutArrived {
                movement_id,
                army_id,
                ..
            } => (movement_id, army_id),
            _ => unreachable!("project_army_arrived called with non-arrival event"),
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
