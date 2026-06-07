//! Army and movement read-model projection dispatcher and shared helpers.
//!
//! This module is projector-specific. It keeps persistence writes in
//! infrastructure, uses `Army` domain helpers when deriving army state, and
//! materializes movement rows from canonical facts.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{MovementDirection, MovementType, VillageMovement};
use parabellum_game::models::army::Army;
use parabellum_types::army::TroopSet;
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;

mod attacks;
mod reinforcements;
mod returns;

struct MovementPairProjection<'a> {
    movement_id: uuid::Uuid,
    movement_type: MovementType,
    player_id: uuid::Uuid,
    source_village_id: u32,
    target_village_id: u32,
    arrives_at: chrono::DateTime<chrono::Utc>,
    army: &'a Army,
}

impl VillageProjector {
    pub(super) async fn project_army_event_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
        aggregate_id: &str,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::VillageArmyDetached { .. } => Some(
                self.project_village_army_detached(tx, aggregate_id, event)
                    .await,
            ),
            VillageEvent::ReinforcementSent { .. } => {
                Some(self.project_reinforcement_sent(tx, event).await)
            }
            VillageEvent::ReinforcementArrived { .. } => {
                Some(self.project_reinforcement_arrived(tx, event).await)
            }
            VillageEvent::ReinforcementAppliedToVillage { .. } => Some(
                self.project_reinforcement_applied_to_village(tx, event)
                    .await,
            ),
            VillageEvent::ReinforcementsRecalled { .. } => {
                Some(self.project_reinforcements_recalled(tx, event).await)
            }
            VillageEvent::ReinforcementsReleased { .. } => {
                Some(self.project_reinforcements_released(tx, event).await)
            }
            VillageEvent::AttackSent { .. } => Some(self.project_attack_sent(tx, event).await),
            VillageEvent::AttackArrivalScheduled { .. } => {
                Some(self.project_attack_arrival_scheduled(tx, event).await)
            }
            VillageEvent::TroopMovementCanceled { .. } => {
                Some(self.project_troop_movement_canceled(tx, event).await)
            }
            VillageEvent::ScoutSent { .. } => Some(self.project_scout_sent(tx, event).await),
            VillageEvent::AttackArrived { .. } | VillageEvent::ScoutArrived { .. } => {
                Some(self.project_army_arrived(tx, event).await)
            }
            VillageEvent::AttackBattleResolved { .. } => {
                Some(self.project_attack_battle_resolved(tx, event).await)
            }
            VillageEvent::ScoutBattleResolved { .. } => {
                Some(self.project_scout_battle_resolved(tx, event).await)
            }
            VillageEvent::ArmyReturned { .. } => Some(self.project_army_returned(tx, event).await),
            _ => None,
        }
    }

    async fn project_village_army_detached(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        aggregate_id: &str,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::VillageArmyDetached {
            army: detached_army,
        } = event
        else {
            unreachable!("project_village_army_detached called with non-VillageArmyDetached event");
        };
        let village_id = aggregate_id
            .parse::<u32>()
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let current = self
            .village
            .get_by_village_id_in_tx(tx, village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let current_home_army = self
            .armies
            .get_home_army_in_tx(tx, village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let previous_home_army_id = current_home_army.as_ref().map(|a| a.id);

        let next_army = if let Some(mut home_army) = current_home_army {
            let detached_hero_id = detached_army.hero().map(|hero| hero.id);
            let home_hero_id = home_army.hero().map(|hero| hero.id);
            let hero_id = (detached_hero_id == home_hero_id)
                .then_some(detached_hero_id)
                .flatten();
            if detached_army.units().immensity() > 0 || hero_id.is_some() {
                home_army
                    .split_units(detached_army.units().clone(), hero_id, village_id)
                    .map_err(CqrsError::domain_source)?;
            }
            (home_army.immensity() > 0).then_some(home_army)
        } else {
            None
        };

        if let Some(home_army) = &next_army {
            self.armies
                .upsert_home_in_tx(tx, home_army, current.player_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        } else if let Some(home_army_id) = previous_home_army_id {
            self.armies
                .delete_in_tx(tx, home_army_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }

        self.upsert_moving_army(tx, detached_army, village_id, current.player_id)
            .await
    }

    async fn upsert_moving_army(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        army: &Army,
        village_id: u32,
        player_id: uuid::Uuid,
    ) -> Result<(), CqrsError> {
        self.armies
            .upsert_moving_in_tx(tx, army, village_id, player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        if let Some(hero) = army.hero() {
            self.heroes
                .upsert_in_tx(tx, &hero, hero.village_id, village_id, "moving")
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }
        self.refresh_village_derived_state_in_tx(tx, army.village_id)
            .await
    }

    async fn upsert_movement_pair(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        projection: MovementPairProjection<'_>,
    ) -> Result<(), CqrsError> {
        let outgoing = VillageMovement {
            movement_id: projection.movement_id,
            movement_type: projection.movement_type,
            direction: MovementDirection::Outgoing,
            origin_village_id: projection.source_village_id,
            origin_village_name: None,
            origin_player_id: projection.player_id,
            origin_position: None,
            target_village_id: projection.target_village_id,
            target_village_name: None,
            target_player_id: None,
            target_position: None,
            arrives_at: projection.arrives_at,
            time_seconds: None,
            units: projection.army.units().clone(),
            tribe: None,
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
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    fn remaining_after_split(
        army: &Army,
        requested: TroopSet,
        carry_hero: bool,
        current_village_id: u32,
    ) -> Result<Option<Army>, CqrsError> {
        let mut remaining = army.clone();
        let hero_id = carry_hero
            .then(|| army.hero().map(|hero| hero.id))
            .flatten();
        remaining
            .split_units(requested, hero_id, current_village_id)
            .map_err(CqrsError::domain_source)?;

        Ok((remaining.immensity() > 0).then_some(remaining))
    }
}
