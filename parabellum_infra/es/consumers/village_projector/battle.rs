//! Battle target read-model projection.
//!
//! This module owns battle outcome materialization that replaces target village
//! state, synchronizes army read models, and applies conquest-side occupancy.

use std::collections::{BTreeSet, HashMap, HashSet};

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::VillageModel;
use parabellum_game::models::army::Army;
use parabellum_game::models::buildings::get_building_data;
use parabellum_game::models::village::Village;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::es::consumers::village_projector::VillageProjector;

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

        let mut target_next = target_state_after_battle(&target_before, event);

        self.sync_target_armies(tx, *target_village_id, &target_before, &target_next)
            .await?;

        if *target_player_id != target_before.player_id {
            self.apply_conquest_to_target(
                tx,
                *source_village_id,
                *target_village_id,
                *target_player_id,
                &mut target_next,
            )
            .await?;
        }

        self.village
            .replace_village_state_in_tx(tx, &target_next)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        self.sync_home_deployed_armies(
            tx,
            &target_before.reinforcements,
            &target_next.reinforcements,
        )
        .await
    }

    async fn sync_target_armies(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        target_village_id: u32,
        target_before: &VillageModel,
        target_next: &VillageModel,
    ) -> Result<(), CqrsError> {
        let before_ids = target_army_ids(target_before);
        let mut after_ids: HashSet<Uuid> = HashSet::new();
        if let Some(after_home) = target_next.army.as_ref() {
            self.armies
                .upsert_home_in_tx(tx, after_home, target_next.player_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            if let Some(hero) = after_home.hero() {
                self.heroes
                    .upsert_in_tx(tx, &hero, hero.village_id, target_village_id, "home")
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            after_ids.insert(after_home.id);
        }
        for after_reinforcement in &target_next.reinforcements {
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
                self.heroes
                    .upsert_in_tx(tx, &hero, hero.village_id, target_village_id, "stationed")
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
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
        let mut conquered_village = Self::village_from_model(target_next);
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
        self.village
            .set_map_occupancy_in_tx(
                tx,
                target_village_id,
                Some(target_village_id),
                Some(target_player_id),
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn sync_home_deployed_armies(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        target_before_reinforcements: &[Army],
        target_next_reinforcements: &[Army],
    ) -> Result<(), CqrsError> {
        let before_by_home = reinforcement_ids_by_home(target_before_reinforcements);
        let after_by_home = reinforcements_by_home(target_next_reinforcements);
        let homes = changed_reinforcement_homes(&before_by_home, &after_by_home);
        for home_village_id in homes {
            let home = self
                .village
                .get_by_village_id_in_tx(tx, home_village_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            let mut next_deployed = home.deployed_armies.clone();
            let before_ids = before_by_home
                .get(&home_village_id)
                .cloned()
                .unwrap_or_default();
            for removed in before_ids {
                if let Some(pos) = next_deployed.iter().position(|army| army.id == removed) {
                    next_deployed.remove(pos);
                }
            }
            let after_armies = after_by_home
                .get(&home_village_id)
                .cloned()
                .unwrap_or_default();
            for updated in after_armies {
                if let Some(pos) = next_deployed.iter().position(|army| army.id == updated.id) {
                    next_deployed[pos] = updated;
                } else {
                    next_deployed.push(updated);
                }
            }
            self.village
                .update_deployed_armies_in_tx(tx, home_village_id, &next_deployed)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }
        Ok(())
    }
}

fn target_army_ids(target: &VillageModel) -> HashSet<Uuid> {
    let mut ids = HashSet::new();
    if let Some(home) = target.army.as_ref() {
        ids.insert(home.id);
    }
    ids.extend(target.reinforcements.iter().map(|army| army.id));
    ids
}

fn reinforcement_ids_by_home(reinforcements: &[Army]) -> HashMap<u32, Vec<Uuid>> {
    let mut by_home: HashMap<u32, Vec<Uuid>> = HashMap::new();
    for reinforcement in reinforcements {
        by_home
            .entry(reinforcement.village_id)
            .or_default()
            .push(reinforcement.id);
    }
    by_home
}

fn reinforcements_by_home(reinforcements: &[Army]) -> HashMap<u32, Vec<Army>> {
    let mut by_home: HashMap<u32, Vec<Army>> = HashMap::new();
    for reinforcement in reinforcements {
        by_home
            .entry(reinforcement.village_id)
            .or_default()
            .push(reinforcement.clone());
    }
    by_home
}

fn changed_reinforcement_homes(
    before_by_home: &HashMap<u32, Vec<Uuid>>,
    after_by_home: &HashMap<u32, Vec<Army>>,
) -> BTreeSet<u32> {
    let mut homes = BTreeSet::new();
    homes.extend(before_by_home.keys().copied());
    homes.extend(after_by_home.keys().copied());
    homes
}

fn target_state_after_battle(target_before: &VillageModel, event: &VillageEvent) -> VillageModel {
    let VillageEvent::BattleOutcomeAppliedToVillage {
        target_player_id,
        target_parent_village_id,
        target_loyalty,
        target_buildings,
        target_production,
        target_population,
        target_stocks,
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

    let mut target_next = target_before.clone();
    target_next.player_id = *target_player_id;
    target_next.parent_village_id = *target_parent_village_id;
    target_next.loyalty = *target_loyalty;
    target_next.buildings = target_buildings.clone();
    target_next.production = target_production.clone();
    target_next.population = *target_population;
    target_next.stocks = target_stocks.clone();
    target_next.army = target_army.clone().filter(|army| army.immensity() > 0);
    target_next.reinforcements = target_reinforcements
        .iter()
        .filter(|army| army.immensity() > 0)
        .cloned()
        .collect();
    if let Some(stationed_attacker) = stationed_attacker_army
        && stationed_attacker.immensity() > 0
    {
        target_next.reinforcements.push(stationed_attacker.clone());
    }

    target_next
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
