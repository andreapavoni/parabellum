//! Returning army and battle-return projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    MovementDirection, MovementType, ScheduledActionStatus, VillageMovement,
};
use parabellum_game::models::army::Army;
use parabellum_types::common::ResourceGroup;
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;
use crate::es::workflows;

struct ReturnProjection<'a> {
    movement_id: uuid::Uuid,
    action_id: uuid::Uuid,
    player_id: uuid::Uuid,
    source_village_id: u32,
    target_village_id: u32,
    returns_at: chrono::DateTime<chrono::Utc>,
    army: &'a Army,
    bounty: Option<ResourceGroup>,
}

impl VillageProjector {
    pub(super) async fn project_attack_battle_resolved(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::AttackBattleResolved {
            movement_id,
            return_action_id,
            player_id,
            source_village_id,
            target_village_id,
            report,
            returning_army,
            stationed_attacker_army,
            returns_at,
            ..
        } = event
        else {
            unreachable!(
                "project_attack_battle_resolved called with non-AttackBattleResolved event"
            );
        };
        let _ = stationed_attacker_army;

        let Some(return_army) = returning_army else {
            return Ok(());
        };
        self.project_battle_return(
            tx,
            ReturnProjection {
                movement_id: *movement_id,
                action_id: *return_action_id,
                player_id: *player_id,
                source_village_id: *source_village_id,
                target_village_id: *target_village_id,
                returns_at: *returns_at,
                army: return_army,
                bounty: report.bounty.clone(),
            },
        )
        .await
    }

    pub(super) async fn project_scout_battle_resolved(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::ScoutBattleResolved {
            movement_id,
            return_action_id,
            player_id,
            source_village_id,
            target_village_id,
            returning_army,
            returns_at,
            ..
        } = event
        else {
            unreachable!("project_scout_battle_resolved called with non-ScoutBattleResolved event");
        };
        let Some(return_army) = returning_army else {
            return Ok(());
        };
        self.project_battle_return(
            tx,
            ReturnProjection {
                movement_id: *movement_id,
                action_id: *return_action_id,
                player_id: *player_id,
                source_village_id: *source_village_id,
                target_village_id: *target_village_id,
                returns_at: *returns_at,
                army: return_army,
                bounty: None,
            },
        )
        .await
    }

    async fn project_battle_return(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        projection: ReturnProjection<'_>,
    ) -> Result<(), CqrsError> {
        let outgoing = VillageMovement {
            movement_id: projection.movement_id,
            movement_type: MovementType::Return,
            direction: MovementDirection::Outgoing,
            origin_village_id: projection.target_village_id,
            origin_village_name: None,
            origin_player_id: projection.player_id,
            origin_position: None,
            target_village_id: projection.source_village_id,
            target_village_name: None,
            target_player_id: None,
            target_position: None,
            arrives_at: projection.returns_at,
            time_seconds: None,
            units: projection.army.units().clone(),
            tribe: Some(projection.army.tribe.clone()),
            bounty: projection.bounty.clone(),
        };
        let incoming = VillageMovement {
            direction: MovementDirection::Incoming,
            ..outgoing.clone()
        };
        self.movements
            .upsert_in_tx(tx, &outgoing)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.movements
            .upsert_in_tx(tx, &incoming)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.upsert_moving_army(
            tx,
            projection.army,
            projection.target_village_id,
            projection.player_id,
        )
        .await?;
        let workflow = workflows::movements::army_return_workflow(
            projection.movement_id,
            projection.army.id,
            projection.source_village_id,
            projection.source_village_id,
            projection.target_village_id,
            projection.player_id,
            projection.army.clone(),
            projection.bounty,
            projection.returns_at,
        );
        let action = workflows::movements::army_return_scheduled_action_from_workflow(
            projection.action_id,
            workflow,
        )?;
        self.add_scheduled_action_in_tx(tx, &action).await
    }

    pub(super) async fn project_troop_movement_canceled(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::TroopMovementCanceled {
            movement_id,
            arrival_action_id,
            return_action_id,
            army_id,
            player_id,
            source_village_id,
            target_village_id,
            army,
            returns_at,
        } = event
        else {
            unreachable!(
                "project_troop_movement_canceled called with non-TroopMovementCanceled event"
            );
        };

        self.actions
            .update_status_in_tx(tx, *arrival_action_id, ScheduledActionStatus::Completed)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        self.movements
            .delete_by_movement_id_in_tx(tx, *movement_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let outgoing = VillageMovement {
            movement_id: *movement_id,
            movement_type: MovementType::Return,
            direction: MovementDirection::Outgoing,
            origin_village_id: *target_village_id,
            origin_village_name: None,
            origin_player_id: *player_id,
            origin_position: None,
            target_village_id: *source_village_id,
            target_village_name: None,
            target_player_id: None,
            target_position: None,
            arrives_at: *returns_at,
            time_seconds: None,
            units: army.units().clone(),
            tribe: Some(army.tribe.clone()),
            bounty: None,
        };
        let incoming = VillageMovement {
            direction: MovementDirection::Incoming,
            ..outgoing.clone()
        };
        self.movements
            .upsert_in_tx(tx, &outgoing)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.movements
            .upsert_in_tx(tx, &incoming)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        self.upsert_moving_army(tx, army, *target_village_id, *player_id)
            .await?;

        let workflow = workflows::movements::army_return_workflow(
            *movement_id,
            *army_id,
            *source_village_id,
            *source_village_id,
            *target_village_id,
            *player_id,
            army.clone(),
            None,
            *returns_at,
        );
        let action = workflows::movements::army_return_scheduled_action_from_workflow(
            *return_action_id,
            workflow,
        )?;
        self.add_scheduled_action_in_tx(tx, &action).await
    }

    pub(super) async fn project_army_returned(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::ArmyReturned {
            movement_id,
            source_village_id,
            army,
            bounty,
            player_id,
            ..
        } = event
        else {
            unreachable!("project_army_returned called with non-ArmyReturned event");
        };
        let source = self
            .village
            .get_by_village_id_in_tx(tx, *source_village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let source_stocks = source.stocks.clone();
        let mut source_village = self
            .village_from_model_with_armies_in_tx(tx, source)
            .await?;
        source_village
            .merge_army(army)
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let next_source_army = source_village
            .army()
            .cloned()
            .filter(|army| army.immensity() > 0);
        self.movements
            .delete_by_movement_id_in_tx(tx, *movement_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.armies
            .delete_in_tx(tx, *movement_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.armies
            .delete_in_tx(tx, army.id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        if let Some(ref home_army) = next_source_army {
            self.armies
                .upsert_home_in_tx(tx, home_army, *player_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            if let Some(hero) = home_army.hero() {
                self.heroes
                    .upsert_in_tx(tx, &hero, *source_village_id, *source_village_id, "home")
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
        }

        if let Some(bounty) = bounty {
            let next_resources = ResourceGroup::new(
                source_stocks.lumber.saturating_add(bounty.lumber()),
                source_stocks.clay.saturating_add(bounty.clay()),
                source_stocks.iron.saturating_add(bounty.iron()),
                (source_stocks.crop.max(0) as u32).saturating_add(bounty.crop()),
            );
            self.set_stored_resources_in_tx(tx, *source_village_id, next_resources)
                .await?;
        }
        self.refresh_village_derived_state_in_tx(tx, *source_village_id)
            .await
    }
}
