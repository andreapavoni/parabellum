//! Reinforcement movement and stationed-army projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{MovementDirection, MovementType, VillageMovement};
use parabellum_app::villages::projection_repositories::{ArmyListFilter, ArmyState};
use parabellum_game::models::army::Army;
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;
use crate::es::consumers::village_projector::armies::MovementPairProjection;
use crate::es::workflows;

impl VillageProjector {
    pub(super) async fn project_reinforcement_sent(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::ReinforcementSent {
            movement_id,
            player_id,
            source_village_id,
            target_village_id,
            army,
            arrives_at,
            ..
        } = event
        else {
            unreachable!("project_reinforcement_sent called with non-ReinforcementSent event");
        };
        self.upsert_moving_army(tx, army, *source_village_id, *player_id)
            .await?;
        self.upsert_movement_pair(
            tx,
            MovementPairProjection {
                movement_id: *movement_id,
                movement_type: MovementType::Reinforcement,
                player_id: *player_id,
                source_village_id: *source_village_id,
                target_village_id: *target_village_id,
                arrives_at: *arrives_at,
                army,
            },
        )
        .await?;

        let action =
            workflows::movements::reinforcement_arrival_scheduled_action_from_event(event)?;
        self.add_scheduled_action_in_tx(tx, &action).await
    }

    pub(super) async fn project_reinforcement_arrived(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::ReinforcementArrived {
            movement_id,
            army_id,
            source_village_id,
            army,
            hero_alone_transfer,
            ..
        } = event
        else {
            unreachable!(
                "project_reinforcement_arrived called with non-ReinforcementArrived event"
            );
        };
        let _ = (source_village_id, army, hero_alone_transfer);
        self.movements
            .delete_by_movement_id_in_tx(tx, *movement_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.armies
            .delete_in_tx(tx, *army_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.refresh_village_derived_state_in_tx(tx, *source_village_id)
            .await
    }

    pub(super) async fn project_reinforcement_applied_to_village(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::ReinforcementAppliedToVillage {
            army_id,
            source_village_id,
            target_village_id,
            player_id,
            army,
            hero_alone_transfer,
            ..
        } = event
        else {
            unreachable!(
                "project_reinforcement_applied_to_village called with non-ReinforcementAppliedToVillage event"
            );
        };
        let target = self
            .village
            .get_by_village_id_in_tx(tx, *target_village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        if *hero_alone_transfer {
            let mut target_home_armies = self
                .armies
                .list_armies_in_tx(
                    tx,
                    ArmyListFilter::new()
                        .home_village(*target_village_id)
                        .current_village(*target_village_id)
                        .state(ArmyState::Home)
                        .limit(1),
                )
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            let mut target_army = target_home_armies
                .pop()
                .unwrap_or_else(|| Army::new_village_army(&Self::village_from_model(&target)));
            target_army.set_hero(army.hero());
            let next_target_army = if target_army.immensity() == 0 {
                None
            } else {
                Some(target_army)
            };
            if let Some(ref home_army) = next_target_army {
                self.armies
                    .upsert_home_in_tx(tx, home_army, target.player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            if let Some(mut hero) = army.hero() {
                hero.village_id = *target_village_id;
                self.heroes
                    .upsert_in_tx(tx, &hero, *target_village_id, *target_village_id, "home")
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
        } else {
            self.merge_stationed_reinforcement(
                tx,
                *army_id,
                *source_village_id,
                *target_village_id,
                *player_id,
                army,
            )
            .await?;
        }
        self.refresh_village_derived_state_in_tx(tx, *target_village_id)
            .await
    }

    async fn merge_stationed_reinforcement(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        arriving_army_id: uuid::Uuid,
        source_village_id: u32,
        target_village_id: u32,
        player_id: uuid::Uuid,
        army: &Army,
    ) -> Result<(), CqrsError> {
        let mut stationed = self
            .armies
            .list_armies_in_tx(
                tx,
                ArmyListFilter::new()
                    .home_village(source_village_id)
                    .current_village(target_village_id)
                    .state(ArmyState::Stationed),
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let mut target_army = stationed.pop().unwrap_or_else(|| army.clone());
        if target_army.id != army.id {
            target_army
                .merge(army)
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            if army.hero().is_some() {
                target_army.set_hero(army.hero());
            }
        }

        for redundant in stationed {
            target_army
                .merge(&redundant)
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            if redundant.hero().is_some() {
                target_army.set_hero(redundant.hero());
            }
            self.armies
                .delete_in_tx(tx, redundant.id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }

        if target_army.id != arriving_army_id {
            self.armies
                .delete_in_tx(tx, arriving_army_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }

        self.armies
            .upsert_stationed_in_tx(tx, &target_army, target_village_id, player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        if let Some(hero) = target_army.hero() {
            self.heroes
                .upsert_in_tx(tx, &hero, hero.village_id, target_village_id, "stationed")
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }

        Ok(())
    }

    pub(super) async fn project_reinforcements_recalled(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        self.project_reinforcement_return(tx, event, true).await
    }

    pub(super) async fn project_reinforcements_released(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        self.project_reinforcement_return(tx, event, false).await
    }

    async fn project_reinforcement_return(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
        visible_to_stationed_village: bool,
    ) -> Result<(), CqrsError> {
        let (
            action_id,
            movement_id,
            army_id,
            player_id,
            home_village_id,
            stationed_village_id,
            army,
            returns_at,
        ) = match event {
            VillageEvent::ReinforcementsRecalled {
                action_id,
                movement_id,
                army_id,
                player_id,
                home_village_id,
                stationed_village_id,
                army,
                returns_at,
            }
            | VillageEvent::ReinforcementsReleased {
                action_id,
                movement_id,
                army_id,
                player_id,
                home_village_id,
                stationed_village_id,
                army,
                returns_at,
            } => (
                action_id,
                movement_id,
                army_id,
                player_id,
                home_village_id,
                stationed_village_id,
                army,
                returns_at,
            ),
            _ => unreachable!("project_reinforcement_return called with non-return event"),
        };

        self.remove_returning_stationed_army(tx, *stationed_village_id, *army_id, army)
            .await?;

        let incoming = VillageMovement {
            movement_id: *movement_id,
            movement_type: MovementType::Return,
            direction: MovementDirection::Incoming,
            origin_village_id: *stationed_village_id,
            origin_village_name: None,
            origin_player_id: army.player_id,
            origin_position: None,
            target_village_id: *home_village_id,
            target_village_name: None,
            target_player_id: None,
            target_position: None,
            arrives_at: *returns_at,
            time_seconds: None,
            units: army.units().clone(),
            has_hero: army.hero().is_some(),
            tribe: Some(army.tribe.clone()),
            bounty: None,
        };
        if visible_to_stationed_village {
            let outgoing = VillageMovement {
                direction: MovementDirection::Outgoing,
                ..incoming.clone()
            };
            self.movements
                .upsert_in_tx(tx, &outgoing)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }
        self.movements
            .upsert_in_tx(tx, &incoming)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.upsert_moving_army(tx, army, *stationed_village_id, army.player_id)
            .await?;

        let workflow = workflows::movements::army_return_workflow(
            *movement_id,
            *army_id,
            *home_village_id,
            *home_village_id,
            *stationed_village_id,
            *player_id,
            army.clone(),
            None,
            *returns_at,
        );
        let action =
            workflows::movements::army_return_scheduled_action_from_workflow(*action_id, workflow)?;
        self.add_scheduled_action_in_tx(tx, &action).await
    }

    async fn remove_returning_stationed_army(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        stationed_village_id: u32,
        army_id: uuid::Uuid,
        returning_army: &Army,
    ) -> Result<(), CqrsError> {
        let Some(existing) = self
            .armies
            .list_armies_in_tx(
                tx,
                ArmyListFilter::new()
                    .army_id(army_id)
                    .current_village(stationed_village_id)
                    .state(ArmyState::Stationed)
                    .limit(1),
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?
            .into_iter()
            .next()
        else {
            return Ok(());
        };

        let remaining = Self::remaining_after_split(
            &existing,
            returning_army.units().clone(),
            returning_army.hero().is_some(),
            stationed_village_id,
        )?;
        if let Some(remaining) = remaining {
            self.armies
                .upsert_stationed_in_tx(tx, &remaining, stationed_village_id, remaining.player_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            if let Some(hero) = remaining.hero() {
                self.heroes
                    .upsert_in_tx(
                        tx,
                        &hero,
                        hero.village_id,
                        stationed_village_id,
                        "stationed",
                    )
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
        } else {
            self.armies
                .delete_in_tx(tx, army_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }
        self.refresh_village_derived_state_in_tx(tx, stationed_village_id)
            .await
    }
}
