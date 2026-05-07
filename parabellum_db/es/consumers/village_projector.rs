//! Synchronous village event projector for ES read models.
//!
//! This consumer runs in the command transaction scope and must keep read-model
//! updates consistent with event appends.
use mini_cqrs_es::{CqrsError, EventConsumer, StoredEvent};
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    MovementDirection, MovementType, ScheduledAction, ScheduledActionPayload,
    ScheduledActionStatus, VillageModel, VillageMovement,
};
use parabellum_app::villages::repositories::{
    MarketplaceOfferRepository, ScheduledActionRepository, VillageModelRepository,
    VillageMovementRepository,
};
use parabellum_game::battle::Battle;
use parabellum_game::models::army::Army;
use parabellum_game::models::buildings::Building;
use parabellum_game::models::smithy::SmithyUpgrades;
use parabellum_game::models::village::Village;
use parabellum_types::army::TroopSet;
use parabellum_types::battle::AttackType;
use parabellum_types::common::ResourceGroup;
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::es::{
    PostgresArmyModelRepository, PostgresMarketplaceOfferRepository,
    PostgresScheduledActionRepository, PostgresVillageModelRepository,
    PostgresVillageMovementRepository,
};

#[derive(Debug, Clone)]
pub struct VillageProjector {
    village: PostgresVillageModelRepository,
    armies: PostgresArmyModelRepository,
    movements: PostgresVillageMovementRepository,
    actions: PostgresScheduledActionRepository,
    offers: PostgresMarketplaceOfferRepository,
}

impl VillageProjector {
    pub fn new(pool: PgPool) -> Self {
        Self {
            village: PostgresVillageModelRepository::new(pool.clone()),
            armies: PostgresArmyModelRepository::new(pool.clone()),
            movements: PostgresVillageMovementRepository::new(pool.clone()),
            actions: PostgresScheduledActionRepository::new(pool.clone()),
            offers: PostgresMarketplaceOfferRepository::new(pool),
        }
    }

    fn village_from_model(model: &VillageModel) -> Village {
        Village::try_from(model.clone()).expect("VillageModel to Village conversion must succeed")
    }

    fn split_army(army: &Army, requested: &TroopSet) -> (Option<Army>, Army) {
        let mut returning = army.clone();
        returning.update_units(requested);

        let mut remaining_units = army.units().clone();
        for idx in 0..10 {
            remaining_units.remove(idx, requested.get(idx));
        }

        if remaining_units.immensity() == 0 {
            (None, returning)
        } else {
            let mut remaining = army.clone();
            remaining.update_units(&remaining_units);
            (Some(remaining), returning)
        }
    }
}

impl EventConsumer for VillageProjector {
    async fn process(&self, event: &StoredEvent) -> Result<(), CqrsError> {
        if !event.aggregate_type.contains("VillageAggregate") {
            return Ok(());
        }

        let domain_event = event.get_payload::<VillageEvent>()?;
        // Projection contract by event family:
        // - founded/conquered/resources/buildings -> rm_village
        // - reinforcement -> rm_village_movements + rm_scheduled_actions
        // - training/research scheduling -> rm_scheduled_actions
        match domain_event {
            VillageEvent::VillageFounded {
                village_id,
                village_name,
                position,
                tribe,
                player_id,
                buildings,
                ..
            } => {
                self.village
                    .upsert_from_village(
                        village_id,
                        player_id,
                        &village_name,
                        &position,
                        tribe,
                        &buildings,
                        &None,
                    )
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .set_map_occupancy(
                        village_id, /* field_id by deterministic map id invariant */
                        Some(village_id),
                        Some(player_id),
                    )
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::VillageConquered { player_id } => {
                let village_id = event
                    .aggregate_id
                    .parse::<u32>()
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .update_player_id(village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::VillageResourcesSet {
                village_id,
                resources,
                ..
            } => {
                self.village
                    .set_stored_resources(village_id, resources)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::VillageArmyDetached {
                army: detached_army,
            } => {
                let village_id = event
                    .aggregate_id
                    .parse::<u32>()
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let current = self
                    .village
                    .get_by_village_id(village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next_army = current.army;
                if let Some(ref mut army) = next_army {
                    let mut next_units = army.units().clone();
                    for idx in 0..10 {
                        next_units.remove(idx, detached_army.units().get(idx));
                    }
                    army.update_units(&next_units);
                    if army.immensity() == 0 {
                        next_army = None;
                    }
                }
                self.village
                    .update_army(village_id, &next_army)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .upsert_moving(&detached_army, village_id, current.player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::ReinforcementSent {
                movement_id,
                army_id,
                player_id,
                source_village_id,
                target_village_id,
                army,
                arrives_at,
            } => {
                self.armies
                    .upsert_moving(&army, source_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let outgoing = VillageMovement {
                    movement_id,
                    movement_type: MovementType::Reinforcement,
                    direction: MovementDirection::Outgoing,
                    origin_village_id: source_village_id,
                    origin_village_name: None,
                    origin_player_id: player_id,
                    origin_position: None,
                    target_village_id,
                    target_village_name: None,
                    target_player_id: None,
                    target_position: None,
                    arrives_at,
                    time_seconds: None,
                    units: army.units().clone(),
                    tribe: None,
                };

                let incoming = VillageMovement {
                    direction: MovementDirection::Incoming,
                    ..outgoing.clone()
                };

                self.movements
                    .upsert(&outgoing)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.movements
                    .upsert(&incoming)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                self.actions
                    .add(&ScheduledAction {
                        id: Uuid::new_v4(),
                        action_type: ScheduledActionPayload::ReinforcementArrival {
                            movement_id,
                            army_id,
                            player_id,
                            source_village_id,
                            target_village_id,
                            army: army.clone(),
                            arrives_at,
                        }
                        .action_type(),
                        execute_at: arrives_at,
                        payload: serde_json::to_value(
                            ScheduledActionPayload::ReinforcementArrival {
                                movement_id,
                                army_id,
                                player_id,
                                source_village_id,
                                target_village_id,
                                army,
                                arrives_at,
                            },
                        )
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::ReinforcementArrived {
                movement_id,
                player_id,
                source_village_id,
                target_village_id,
                army,
                ..
            } => {
                let source = self
                    .village
                    .get_by_village_id(source_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut source_deployed = source.deployed_armies;
                source_deployed.push(army.clone());
                self.village
                    .update_deployed_armies(source_village_id, &source_deployed)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let target = self
                    .village
                    .get_by_village_id(target_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut target_reinforcements = target.reinforcements;
                target_reinforcements.push(army.clone());
                self.village
                    .update_reinforcements(target_village_id, &target_reinforcements)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .upsert_stationed(&army, target_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                self.movements
                    .delete_by_movement_id(movement_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
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
            } => {
                let source = self
                    .village
                    .get_by_village_id(home_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next_deployed = source.deployed_armies;
                if let Some(idx) = next_deployed.iter().position(|a| a.id == army_id) {
                    let existing = next_deployed[idx].clone();
                    let (remaining, _) = Self::split_army(&existing, army.units());
                    if let Some(remaining) = remaining {
                        next_deployed[idx] = remaining;
                    } else {
                        next_deployed.remove(idx);
                    }
                    self.village
                        .update_deployed_armies(home_village_id, &next_deployed)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }

                let stationed = self
                    .village
                    .get_by_village_id(stationed_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next_reinforcements = stationed.reinforcements;
                if let Some(idx) = next_reinforcements.iter().position(|a| a.id == army_id) {
                    let existing = next_reinforcements[idx].clone();
                    let (remaining, _) = Self::split_army(&existing, army.units());
                    if let Some(remaining) = remaining {
                        next_reinforcements[idx] = remaining.clone();
                        self.armies
                            .upsert_stationed(&remaining, stationed_village_id, player_id)
                            .await
                            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    } else {
                        next_reinforcements.remove(idx);
                        self.armies
                            .delete(army_id)
                            .await
                            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    }
                    self.village
                        .update_reinforcements(stationed_village_id, &next_reinforcements)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }

                let outgoing = VillageMovement {
                    movement_id,
                    movement_type: MovementType::Return,
                    direction: MovementDirection::Outgoing,
                    origin_village_id: stationed_village_id,
                    origin_village_name: None,
                    origin_player_id: player_id,
                    origin_position: None,
                    target_village_id: home_village_id,
                    target_village_name: None,
                    target_player_id: None,
                    target_position: None,
                    arrives_at: returns_at,
                    time_seconds: None,
                    units: army.units().clone(),
                    tribe: None,
                };
                let incoming = VillageMovement {
                    direction: MovementDirection::Incoming,
                    ..outgoing.clone()
                };
                self.movements
                    .upsert(&outgoing)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.movements
                    .upsert(&incoming)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .upsert_moving(&army, stationed_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: ScheduledActionPayload::ArmyReturn {
                            action_id,
                            movement_id,
                            army_id,
                            village_id: home_village_id,
                            source_village_id: home_village_id,
                            target_village_id: stationed_village_id,
                            player_id,
                            army: army.clone(),
                            bounty: None,
                            returns_at,
                        }
                        .action_type(),
                        execute_at: returns_at,
                        payload: serde_json::to_value(ScheduledActionPayload::ArmyReturn {
                            action_id,
                            movement_id,
                            army_id,
                            village_id: home_village_id,
                            source_village_id: home_village_id,
                            target_village_id: stationed_village_id,
                            player_id,
                            army,
                            bounty: None,
                            returns_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::SettlersSent {
                action_id,
                movement_id,
                army_id,
                player_id,
                source_village_id,
                target_village_id,
                target_position,
                village_name,
                tribe,
                army,
                arrives_at,
                ..
            } => {
                self.armies
                    .upsert_moving(&army, source_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let source = self
                    .village
                    .get_by_village_id(source_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .set_stored_resources(
                        source_village_id,
                        ResourceGroup::new(
                            source.stocks.lumber.saturating_sub(800),
                            source.stocks.clay.saturating_sub(800),
                            source.stocks.iron.saturating_sub(800),
                            source.stocks.crop.saturating_sub(800).max(0) as u32,
                        ),
                    )
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: ScheduledActionPayload::SettlersArrival {
                            action_id,
                            movement_id,
                            army_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            target_position: target_position.clone(),
                            player_id,
                            village_name: village_name.clone(),
                            tribe: tribe.clone(),
                            arrives_at,
                        }
                        .action_type(),
                        execute_at: arrives_at,
                        payload: serde_json::to_value(ScheduledActionPayload::SettlersArrival {
                            action_id,
                            movement_id,
                            army_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            target_position,
                            player_id,
                            village_name,
                            tribe,
                            arrives_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::SettlersArrived { army_id, .. } => {
                self.armies
                    .delete(army_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::AttackSent {
                movement_id,
                arrival_action_id,
                return_action_id,
                player_id,
                source_village_id,
                target_village_id,
                army_id,
                army,
                attack_type,
                catapult_targets,
                arrives_at,
                returns_at,
            } => {
                self.armies
                    .upsert_moving(&army, source_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let movement_type = match attack_type {
                    AttackType::Raid => MovementType::Raid,
                    AttackType::Normal => MovementType::Attack,
                };
                let outgoing = VillageMovement {
                    movement_id,
                    movement_type,
                    direction: MovementDirection::Outgoing,
                    origin_village_id: source_village_id,
                    origin_village_name: None,
                    origin_player_id: player_id,
                    origin_position: None,
                    target_village_id,
                    target_village_name: None,
                    target_player_id: None,
                    target_position: None,
                    arrives_at,
                    time_seconds: None,
                    units: army.units().clone(),
                    tribe: None,
                };
                let incoming = VillageMovement {
                    direction: MovementDirection::Incoming,
                    ..outgoing.clone()
                };
                self.movements
                    .upsert(&outgoing)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.movements
                    .upsert(&incoming)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                self.actions
                    .add(&ScheduledAction {
                        id: arrival_action_id,
                        action_type: ScheduledActionPayload::AttackArrival {
                            action_id: arrival_action_id,
                            movement_id,
                            return_action_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            army_id,
                            army: army.clone(),
                            attack_type: attack_type.clone(),
                            catapult_targets: catapult_targets.clone(),
                            arrives_at,
                            returns_at,
                        }
                        .action_type(),
                        execute_at: arrives_at,
                        payload: serde_json::to_value(ScheduledActionPayload::AttackArrival {
                            action_id: arrival_action_id,
                            movement_id,
                            return_action_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            army_id,
                            army,
                            attack_type,
                            catapult_targets,
                            arrives_at,
                            returns_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::ScoutSent {
                movement_id,
                arrival_action_id,
                return_action_id,
                player_id,
                source_village_id,
                target_village_id,
                army_id,
                army,
                target,
                attack_type,
                arrives_at,
                returns_at,
            } => {
                self.armies
                    .upsert_moving(&army, source_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let outgoing = VillageMovement {
                    movement_id,
                    movement_type: MovementType::Scout,
                    direction: MovementDirection::Outgoing,
                    origin_village_id: source_village_id,
                    origin_village_name: None,
                    origin_player_id: player_id,
                    origin_position: None,
                    target_village_id,
                    target_village_name: None,
                    target_player_id: None,
                    target_position: None,
                    arrives_at,
                    time_seconds: None,
                    units: army.units().clone(),
                    tribe: None,
                };
                let incoming = VillageMovement {
                    direction: MovementDirection::Incoming,
                    ..outgoing.clone()
                };
                self.movements
                    .upsert(&outgoing)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.movements
                    .upsert(&incoming)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                self.actions
                    .add(&ScheduledAction {
                        id: arrival_action_id,
                        action_type: ScheduledActionPayload::ScoutArrival {
                            action_id: arrival_action_id,
                            movement_id,
                            army_id,
                            return_action_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            army: army.clone(),
                            target: target.clone(),
                            attack_type: attack_type.clone(),
                            arrives_at,
                            returns_at,
                        }
                        .action_type(),
                        execute_at: arrives_at,
                        payload: serde_json::to_value(ScheduledActionPayload::ScoutArrival {
                            action_id: arrival_action_id,
                            movement_id,
                            army_id,
                            return_action_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            army,
                            target,
                            attack_type,
                            arrives_at,
                            returns_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::AttackArrived {
                movement_id,
                army_id,
                action_id: _,
                return_action_id,
                player_id,
                source_village_id,
                target_village_id,
                army,
                attack_type,
                catapult_targets,
                returns_at,
                ..
            } => {
                self.movements
                    .delete_by_movement_id(movement_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .delete(army_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let source = self
                    .village
                    .get_by_village_id(source_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let target = self
                    .village
                    .get_by_village_id(target_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let attacker_village = Self::village_from_model(&source);
                let mut defender_village = Self::village_from_model(&target);
                let no_smithy: SmithyUpgrades = [0; 8];
                let mut attacker_army = Army::new(
                    Some(army_id),
                    army.village_id,
                    army.current_map_field_id,
                    army.player_id,
                    army.tribe.clone(),
                    army.units(),
                    army.smithy(),
                    army.hero(),
                );

                let mut selected_targets: Vec<Building> = Vec::new();
                for name in catapult_targets {
                    match defender_village.get_building_by_name(&name) {
                        Some(slot) => selected_targets.push(slot.building.clone()),
                        None => {
                            if let Some(random) = defender_village.get_random_buildings(1).pop() {
                                selected_targets.push(random);
                            }
                        }
                    }
                }
                let selected_targets = selected_targets.try_into().ok();
                let battle = Battle::new(
                    attack_type,
                    attacker_army.clone(),
                    attacker_village,
                    defender_village.clone(),
                    selected_targets,
                );
                let report = battle.calculate_battle();
                let bounty = report
                    .bounty
                    .clone()
                    .unwrap_or_else(|| ResourceGroup::new(0, 0, 0, 0));

                attacker_army.apply_battle_report(&report.attacker);
                let _ = defender_village.apply_battle_report(&report, 1);

                let mut target_next = target.clone();
                target_next.buildings = defender_village.buildings().to_vec();
                target_next.production = defender_village.production.clone();
                target_next.population = defender_village.population;
                target_next.loyalty = defender_village.loyalty();
                target_next.stocks = defender_village.stocks().clone();
                target_next.army = defender_village.army().cloned();
                target_next.reinforcements = defender_village.reinforcements().clone();
                self.village
                    .replace_village_state(&target_next)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                // Keep rm_armies aligned with defender-side casualties:
                // - home army is projected as `home` when present
                // - stationed reinforcements are projected as `stationed`
                // - wiped armies are removed
                let mut before_ids: HashSet<Uuid> = HashSet::new();
                if let Some(before_home) = target.army.as_ref() {
                    before_ids.insert(before_home.id);
                }
                for before_reinforcement in &target.reinforcements {
                    before_ids.insert(before_reinforcement.id);
                }

                let mut after_ids: HashSet<Uuid> = HashSet::new();
                if let Some(after_home) = target_next.army.as_ref() {
                    self.armies
                        .upsert_home(after_home, target_next.player_id)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    after_ids.insert(after_home.id);
                }
                for after_reinforcement in &target_next.reinforcements {
                    self.armies
                        .upsert_stationed(
                            after_reinforcement,
                            target_village_id,
                            after_reinforcement.player_id,
                        )
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    after_ids.insert(after_reinforcement.id);
                }
                for removed_id in before_ids.difference(&after_ids) {
                    self.armies
                        .delete(*removed_id)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }

                if attacker_army.immensity() == 0 {
                    return Ok(());
                }

                let mut source_deployed = source.deployed_armies;
                let return_army = Army::new(
                    Some(movement_id),
                    source_village_id,
                    Some(target_village_id),
                    player_id,
                    source.tribe.clone(),
                    attacker_army.units(),
                    &no_smithy,
                    None,
                );
                source_deployed.push(return_army.clone());
                self.armies
                    .upsert_moving(&return_army, target_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .update_deployed_armies(source_village_id, &source_deployed)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                self.actions
                    .add(&ScheduledAction {
                        id: return_action_id,
                        action_type: ScheduledActionPayload::ArmyReturn {
                            action_id: return_action_id,
                            movement_id,
                            army_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            army: attacker_army.clone(),
                            bounty: Some(bounty.clone()),
                            returns_at,
                        }
                        .action_type(),
                        execute_at: returns_at,
                        payload: serde_json::to_value(ScheduledActionPayload::ArmyReturn {
                            action_id: return_action_id,
                            movement_id,
                            army_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            army: attacker_army,
                            bounty: Some(bounty),
                            returns_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::ScoutArrived {
                movement_id,
                army_id,
                return_action_id,
                player_id,
                source_village_id,
                target_village_id,
                army,
                target,
                attack_type,
                returns_at,
                ..
            } => {
                self.movements
                    .delete_by_movement_id(movement_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .delete(army_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let source = self
                    .village
                    .get_by_village_id(source_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let target_village_model = self
                    .village
                    .get_by_village_id(target_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let attacker_village = Self::village_from_model(&source);
                let defender_village = Self::village_from_model(&target_village_model);
                let no_smithy: SmithyUpgrades = [0; 8];
                let mut attacker_army = Army::new(
                    Some(army_id),
                    army.village_id,
                    army.current_map_field_id,
                    army.player_id,
                    army.tribe.clone(),
                    army.units(),
                    army.smithy(),
                    army.hero(),
                );
                let battle = Battle::new(
                    attack_type.clone(),
                    attacker_army.clone(),
                    attacker_village,
                    defender_village,
                    None,
                );
                let report = battle.calculate_scout_battle(target.clone());
                attacker_army.update_units(&report.attacker.survivors);

                if attacker_army.immensity() == 0 {
                    return Ok(());
                }

                let source_deployed = source.deployed_armies;
                let return_army = Army::new(
                    Some(movement_id),
                    source_village_id,
                    Some(target_village_id),
                    player_id,
                    source.tribe.clone(),
                    attacker_army.units(),
                    &no_smithy,
                    None,
                );
                let mut next_deployed = source_deployed;
                next_deployed.push(return_army.clone());
                self.armies
                    .upsert_moving(&return_army, target_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .update_deployed_armies(source_village_id, &next_deployed)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                self.actions
                    .add(&ScheduledAction {
                        id: return_action_id,
                        action_type: ScheduledActionPayload::ArmyReturn {
                            action_id: return_action_id,
                            movement_id,
                            army_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            army: return_army.clone(),
                            bounty: None,
                            returns_at,
                        }
                        .action_type(),
                        execute_at: returns_at,
                        payload: serde_json::to_value(ScheduledActionPayload::ArmyReturn {
                            action_id: return_action_id,
                            movement_id,
                            army_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            army: return_army,
                            bounty: None,
                            returns_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::ArmyReturned {
                movement_id,
                source_village_id,
                army,
                bounty,
                player_id,
                ..
            } => {
                let source = self
                    .village
                    .get_by_village_id(source_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut source_army = source
                    .army
                    .clone()
                    .unwrap_or_else(|| Army::new_village_army(&Self::village_from_model(&source)));
                let mut source_deployed = source.deployed_armies;
                let mut next_units = source_army.units().clone();
                for idx in 0..10 {
                    next_units.add(idx, army.units().get(idx));
                }
                source_army.update_units(&next_units);
                source_deployed.retain(|army| army.id != movement_id);
                let next_source_army = if source_army.immensity() == 0 {
                    None
                } else {
                    Some(source_army)
                };
                self.village
                    .update_army(source_village_id, &next_source_army)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .update_deployed_armies(source_village_id, &source_deployed)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .delete(movement_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .delete(army.id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if let Some(ref home_army) = next_source_army {
                    self.armies
                        .upsert_home(home_army, player_id)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }

                if let Some(bounty) = bounty {
                    let next_resources = parabellum_types::common::ResourceGroup::new(
                        source.stocks.lumber.saturating_add(bounty.lumber()),
                        source.stocks.clay.saturating_add(bounty.clay()),
                        source.stocks.iron.saturating_add(bounty.iron()),
                        (source.stocks.crop.max(0) as u32).saturating_add(bounty.crop()),
                    );
                    self.village
                        .set_stored_resources(source_village_id, next_resources)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
            }
            VillageEvent::MerchantsTripScheduled {
                arrival_action_id,
                return_action_id,
                player_id,
                source_village_id,
                target_village_id,
                resources,
                merchants_used,
                resources_already_reserved,
                arrives_at,
                returns_at,
            } => {
                self.actions
                    .add(&ScheduledAction {
                        id: arrival_action_id,
                        action_type: ScheduledActionPayload::MerchantsArrival {
                            action_id: arrival_action_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            resources: resources.clone(),
                            merchants_used,
                            arrives_at,
                        }
                        .action_type(),
                        execute_at: arrives_at,
                        payload: serde_json::to_value(ScheduledActionPayload::MerchantsArrival {
                            action_id: arrival_action_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            resources: resources.clone(),
                            merchants_used,
                            arrives_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                self.actions
                    .add(&ScheduledAction {
                        id: return_action_id,
                        action_type: ScheduledActionPayload::MerchantsReturn {
                            action_id: return_action_id,
                            village_id: source_village_id,
                            source_village_id,
                            player_id,
                            merchants_used,
                            returns_at,
                        }
                        .action_type(),
                        execute_at: returns_at,
                        payload: serde_json::to_value(ScheduledActionPayload::MerchantsReturn {
                            action_id: return_action_id,
                            village_id: source_village_id,
                            source_village_id,
                            player_id,
                            merchants_used,
                            returns_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                if !resources_already_reserved {
                    let source = self
                        .village
                        .get_by_village_id(source_village_id)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    let next_resources = parabellum_types::common::ResourceGroup::new(
                        source.stocks.lumber.saturating_sub(resources.lumber()),
                        source.stocks.clay.saturating_sub(resources.clay()),
                        source.stocks.iron.saturating_sub(resources.iron()),
                        (source.stocks.crop.max(0) as u32).saturating_sub(resources.crop()),
                    );
                    self.village
                        .set_stored_resources(source_village_id, next_resources)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    let next_busy = source.busy_merchants.saturating_add(merchants_used);
                    self.village
                        .set_busy_merchants(source_village_id, next_busy)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
            }
            VillageEvent::MerchantsArrived {
                target_village_id,
                resources,
                ..
            } => {
                let target = self
                    .village
                    .get_by_village_id(target_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let next_resources = parabellum_types::common::ResourceGroup::new(
                    target.stocks.lumber.saturating_add(resources.lumber()),
                    target.stocks.clay.saturating_add(resources.clay()),
                    target.stocks.iron.saturating_add(resources.iron()),
                    (target.stocks.crop.max(0) as u32).saturating_add(resources.crop()),
                );
                self.village
                    .set_stored_resources(target_village_id, next_resources)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::MerchantsReturned {
                source_village_id,
                merchants_used,
                ..
            } => {
                let source = self
                    .village
                    .get_by_village_id(source_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let next_busy = source.busy_merchants.saturating_sub(merchants_used);
                self.village
                    .set_busy_merchants(source_village_id, next_busy)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::MarketplaceOfferCreated {
                offer_id,
                owner_player_id,
                owner_village_id,
                offer_resources,
                seek_resources,
                merchants_reserved,
                created_at,
            } => {
                self.offers
                    .upsert(&parabellum_app::villages::models::MarketplaceOfferModel {
                        offer_id,
                        owner_player_id,
                        owner_village_id,
                        offer_resources,
                        seek_resources,
                        merchants_reserved,
                        status: parabellum_app::villages::models::MarketplaceOfferStatus::Open,
                        accepted_by_player_id: None,
                        accepted_by_village_id: None,
                        created_at,
                        accepted_at: None,
                        canceled_at: None,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let owner = self
                    .village
                    .get_by_village_id(owner_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let reserved: parabellum_types::common::ResourceGroup = offer_resources.into();
                let next_resources = parabellum_types::common::ResourceGroup::new(
                    owner.stocks.lumber.saturating_sub(reserved.lumber()),
                    owner.stocks.clay.saturating_sub(reserved.clay()),
                    owner.stocks.iron.saturating_sub(reserved.iron()),
                    (owner.stocks.crop.max(0) as u32).saturating_sub(reserved.crop()),
                );
                self.village
                    .set_stored_resources(owner_village_id, next_resources)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .set_busy_merchants(
                        owner_village_id,
                        owner.busy_merchants.saturating_add(merchants_reserved),
                    )
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::MarketplaceOfferCanceled {
                offer_id,
                owner_village_id,
                offer_resources,
                merchants_reserved,
                canceled_at,
                ..
            } => {
                self.offers
                    .set_status(
                        offer_id,
                        parabellum_app::villages::models::MarketplaceOfferStatus::Canceled,
                        None,
                        None,
                        canceled_at,
                    )
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let owner = self
                    .village
                    .get_by_village_id(owner_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let refund: parabellum_types::common::ResourceGroup = offer_resources.into();
                let next_resources = parabellum_types::common::ResourceGroup::new(
                    owner.stocks.lumber.saturating_add(refund.lumber()),
                    owner.stocks.clay.saturating_add(refund.clay()),
                    owner.stocks.iron.saturating_add(refund.iron()),
                    (owner.stocks.crop.max(0) as u32).saturating_add(refund.crop()),
                );
                self.village
                    .set_stored_resources(owner_village_id, next_resources)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .set_busy_merchants(
                        owner_village_id,
                        owner.busy_merchants.saturating_sub(merchants_reserved),
                    )
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::MarketplaceOfferAccepted {
                offer_id,
                accepting_player_id,
                accepting_village_id,
                accepted_at,
                ..
            } => {
                self.offers
                    .set_status(
                        offer_id,
                        parabellum_app::villages::models::MarketplaceOfferStatus::Accepted,
                        Some(accepting_player_id),
                        Some(accepting_village_id),
                        accepted_at,
                    )
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::BuildingConstructionScheduled {
                action_id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                execute_at,
            } => {
                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: ScheduledActionPayload::AddBuilding {
                            village_id,
                            player_id,
                            slot_id,
                            building_name: building_name.clone(),
                            level,
                            speed,
                        }
                        .action_type(),
                        execute_at,
                        payload: serde_json::to_value(ScheduledActionPayload::AddBuilding {
                            village_id,
                            player_id,
                            slot_id,
                            building_name,
                            level,
                            speed,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::BuildingUpgradeScheduled {
                action_id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                execute_at,
            } => {
                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: ScheduledActionPayload::UpgradeBuilding {
                            village_id,
                            player_id,
                            slot_id,
                            building_name: building_name.clone(),
                            level,
                            speed,
                        }
                        .action_type(),
                        execute_at,
                        payload: serde_json::to_value(ScheduledActionPayload::UpgradeBuilding {
                            village_id,
                            player_id,
                            slot_id,
                            building_name,
                            level,
                            speed,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::BuildingDowngradeScheduled {
                action_id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                execute_at,
            } => {
                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: ScheduledActionPayload::DowngradeBuilding {
                            village_id,
                            player_id,
                            slot_id,
                            building_name: building_name.clone(),
                            level,
                            speed,
                        }
                        .action_type(),
                        execute_at,
                        payload: serde_json::to_value(ScheduledActionPayload::DowngradeBuilding {
                            village_id,
                            player_id,
                            slot_id,
                            building_name,
                            level,
                            speed,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
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
            } => {
                self.village
                    .update_building(village_id, slot_id, building_name, level, speed)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::UnitTrainingScheduled {
                action_id,
                player_id,
                village_id,
                slot_id,
                unit,
                time_per_unit,
                quantity_remaining,
                execute_at,
            } => {
                let payload = ScheduledActionPayload::TrainUnit {
                    action_id,
                    village_id,
                    player_id,
                    slot_id,
                    unit,
                    time_per_unit,
                    quantity_remaining,
                    execute_at,
                };
                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: payload.action_type(),
                        execute_at,
                        payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::UnitTrained {
                village_id,
                unit,
                quantity_trained,
                ..
            } => {
                let current = self
                    .village
                    .get_by_village_id(village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next_army = current
                    .army
                    .clone()
                    .unwrap_or_else(|| Army::new_village_army(&Self::village_from_model(&current)));
                if current.tribe.get_unit_idx_by_name(&unit).is_some() {
                    next_army
                        .add_unit(unit, quantity_trained)
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    self.village
                        .update_army(village_id, &Some(next_army))
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
            }
            VillageEvent::AcademyResearchScheduled {
                action_id,
                player_id,
                village_id,
                unit,
                execute_at,
            } => {
                let payload = ScheduledActionPayload::ResearchAcademy {
                    action_id,
                    village_id,
                    player_id,
                    unit,
                };
                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: payload.action_type(),
                        execute_at,
                        payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::AcademyResearchCompleted {
                village_id, unit, ..
            } => {
                let current = self
                    .village
                    .get_by_village_id(village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut village = Self::village_from_model(&current);
                village
                    .research_academy(unit)
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let mut next = current.clone();
                next.academy_research = village.academy_research().clone();
                self.village
                    .replace_village_state(&next)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::SmithyResearchScheduled {
                action_id,
                player_id,
                village_id,
                unit,
                execute_at,
            } => {
                let payload = ScheduledActionPayload::ResearchSmithy {
                    action_id,
                    village_id,
                    player_id,
                    unit,
                };
                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: payload.action_type(),
                        execute_at,
                        payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::SmithyResearchCompleted {
                village_id, unit, ..
            } => {
                let current = self
                    .village
                    .get_by_village_id(village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut village = Self::village_from_model(&current);
                village
                    .upgrade_smithy(unit)
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let mut next = current.clone();
                next.smithy_upgrades = *village.smithy();
                self.village
                    .replace_village_state(&next)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
        }

        Ok(())
    }
}
