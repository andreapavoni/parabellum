//! Battle target read-model projection.
//!
//! This module owns battle outcome materialization that replaces target village
//! state, synchronizes army read models, and applies conquest-side occupancy.

use std::collections::HashSet;

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::VillageModel;
use parabellum_app::villages::projection_repositories::{
    ArmyListFilter, ArmyState, HeroPlacementState,
};
use parabellum_game::models::army::Army;
use parabellum_game::models::buildings::get_building_data;
use parabellum_game::models::village::Village;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::es::consumers::village_projector::VillageProjector;

struct BattleTargetState {
    village: VillageModel,
    home: Option<Army>,
    stationed: Vec<Army>,
}

impl VillageProjector {
    pub(super) async fn project_battle_event_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::BattleOutcomeAppliedToVillage { .. } => Some(
                self.project_battle_outcome_applied_to_village(tx, event)
                    .await,
            ),
            _ => None,
        }
    }

    async fn project_battle_outcome_applied_to_village(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::BattleOutcomeAppliedToVillage {
            source_village_id,
            target_village_id,
            target_player_id,
            ..
        } = event
        else {
            unreachable!(
                "project_battle_outcome_applied_to_village called with non-BattleOutcomeAppliedToVillage event"
            );
        };

        let target_before = self
            .village
            .get_by_village_id_in_tx(tx, *target_village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let target_before_army_ids = self.target_army_ids(tx, *target_village_id).await?;
        let mut target_next = target_state_after_battle(&target_before, event);

        self.sync_target_armies(
            tx,
            *target_village_id,
            &target_before_army_ids,
            &target_next,
        )
        .await?;

        if *target_player_id != target_before.player_id {
            self.apply_conquest_to_target(
                tx,
                *source_village_id,
                *target_village_id,
                *target_player_id,
                &mut target_next.village,
            )
            .await?;
        }

        self.village
            .store_village_model_in_tx(tx, &target_next.village)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn sync_target_armies(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        target_village_id: u32,
        before_ids: &HashSet<Uuid>,
        target_next: &BattleTargetState,
    ) -> Result<(), CqrsError> {
        let mut after_ids: HashSet<Uuid> = HashSet::new();
        if let Some(after_home) = target_next.home.as_ref() {
            self.armies
                .upsert_home_in_tx(tx, after_home, target_next.village.player_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            if let Some(hero) = after_home.hero() {
                self.project_hero_placement_in_tx(
                    tx,
                    &hero,
                    target_village_id,
                    target_village_id,
                    HeroPlacementState::Home,
                )
                .await?;
            }
            after_ids.insert(after_home.id);
        }
        for after_reinforcement in &target_next.stationed {
            self.armies
                .upsert_stationed_in_tx(
                    tx,
                    after_reinforcement,
                    target_village_id,
                    after_reinforcement.player_id,
                )
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            if let Some(hero) = after_reinforcement.hero() {
                self.project_hero_placement_in_tx(
                    tx,
                    &hero,
                    hero.village_id,
                    target_village_id,
                    HeroPlacementState::Stationed,
                )
                .await?;
            }
            after_ids.insert(after_reinforcement.id);
        }
        for removed_id in before_ids.difference(&after_ids) {
            self.armies
                .delete_in_tx(tx, *removed_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }
        Ok(())
    }

    async fn target_army_ids(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        target_village_id: u32,
    ) -> Result<HashSet<Uuid>, CqrsError> {
        let mut ids = HashSet::new();
        let mut home_armies = self
            .armies
            .list_armies_in_tx(
                tx,
                ArmyListFilter::new()
                    .home_village(target_village_id)
                    .current_village(target_village_id)
                    .state(ArmyState::Home)
                    .limit(1),
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        if let Some(home) = home_armies.pop() {
            ids.insert(home.id);
        }
        ids.extend(
            self.armies
                .list_armies_in_tx(
                    tx,
                    ArmyListFilter::new()
                        .current_village(target_village_id)
                        .state(ArmyState::Stationed),
                )
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?
                .into_iter()
                .map(|army| army.id),
        );
        Ok(ids)
    }

    async fn apply_conquest_to_target(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        source_village_id: u32,
        target_village_id: u32,
        target_player_id: Uuid,
        target_next: &mut VillageModel,
    ) -> Result<(), CqrsError> {
        let source = self
            .village
            .get_by_village_id_in_tx(tx, source_village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        target_next.tribe = source.tribe.clone();
        let mut conquered_village = self
            .load_village_state_in_tx(tx, target_next.clone())
            .await?;
        conquered_village.tribe = source.tribe;
        remove_tribe_incompatible_buildings(&mut conquered_village);
        target_next.buildings = conquered_village.buildings().to_vec();
        target_next.production = conquered_village.production.clone();
        target_next.population = conquered_village.population;
        target_next.stocks = conquered_village.stocks().clone();
        self.armies
            .delete_by_home_village_in_tx(tx, target_village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.map
            .set_occupancy_in_tx(
                tx,
                target_village_id,
                Some(target_village_id),
                Some(target_player_id),
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }
}

fn target_state_after_battle(
    target_before: &VillageModel,
    event: &VillageEvent,
) -> BattleTargetState {
    let VillageEvent::BattleOutcomeAppliedToVillage {
        target_player_id,
        target_parent_village_id,
        target_loyalty,
        target_buildings,
        target_production,
        target_population,
        target_stocks,
        target_trapper,
        target_army,
        target_reinforcements,
        stationed_attacker_army,
        ..
    } = event
    else {
        unreachable!(
            "target_state_after_battle called with non-BattleOutcomeAppliedToVillage event"
        );
    };

    let mut village = target_before.clone();
    village.player_id = *target_player_id;
    village.parent_village_id = *target_parent_village_id;
    village.loyalty = *target_loyalty;
    village.buildings = target_buildings.clone();
    village.production = target_production.clone();
    village.population = *target_population;
    village.stocks = target_stocks.clone();
    village.trapper = *target_trapper;
    let home = target_army
        .clone()
        .map(army_without_dead_hero)
        .filter(|army| army.immensity() > 0);
    let mut stationed: Vec<Army> = target_reinforcements
        .iter()
        .cloned()
        .map(army_without_dead_hero)
        .filter(|army| army.immensity() > 0)
        .collect();
    if let Some(stationed_attacker) = stationed_attacker_army {
        let stationed_attacker = army_without_dead_hero(stationed_attacker.clone());
        if stationed_attacker.immensity() > 0 {
            stationed.push(stationed_attacker);
        }
    }
    BattleTargetState {
        village,
        home,
        stationed,
    }
}

fn army_without_dead_hero(mut army: Army) -> Army {
    army.detach_dead_hero();
    army
}

fn remove_tribe_incompatible_buildings(village: &mut Village) {
    let tribe = village.tribe.clone();
    let incompatible_slots: Vec<u8> = village
        .buildings()
        .iter()
        .filter_map(|vb| {
            let data = get_building_data(&vb.building.name).ok()?;
            if data.rules.tribes.is_empty() || data.rules.tribes.contains(&tribe) {
                None
            } else {
                Some(vb.slot_id)
            }
        })
        .collect();
    for slot_id in incompatible_slots {
        let _ = village.remove_building_at_slot(slot_id, 1);
    }
}
