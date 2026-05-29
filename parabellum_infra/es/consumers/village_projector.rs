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
use parabellum_app::villages::repositories::VillageRepository;
use parabellum_game::models::army::Army;
use parabellum_game::models::buildings::get_building_data;
use parabellum_game::models::village::Village;
use parabellum_types::army::TroopSet;
use parabellum_types::battle::AttackType;
use parabellum_types::common::ResourceGroup;
use parabellum_types::buildings::BuildingName;
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashSet;
use uuid::Uuid;

use crate::es::{
    PostgresArmyRepository, PostgresHeroRepository, PostgresMarketplaceRepository,
    PostgresScheduledActionRepository, PostgresVillageMovementRepository,
    PostgresVillageRepository,
};

#[derive(Clone)]
pub struct VillageProjector {
    pool: PgPool,
    village: PostgresVillageRepository,
    armies: PostgresArmyRepository,
    heroes: PostgresHeroRepository,
    movements: PostgresVillageMovementRepository,
    actions: PostgresScheduledActionRepository,
    offers: PostgresMarketplaceRepository,
    project_operational_actions: bool,
}

impl VillageProjector {
    pub fn new(pool: PgPool) -> Self {
        Self::new_with_options(pool, true)
    }

    pub fn new_with_options(pool: PgPool, project_operational_actions: bool) -> Self {
        Self {
            pool: pool.clone(),
            village: PostgresVillageRepository::new(pool.clone()),
            armies: PostgresArmyRepository::new(pool.clone()),
            heroes: PostgresHeroRepository::new(pool.clone()),
            movements: PostgresVillageMovementRepository::new(pool.clone()),
            actions: PostgresScheduledActionRepository::new(pool.clone()),
            offers: PostgresMarketplaceRepository::new(pool),
            project_operational_actions,
        }
    }

    fn village_from_model(model: &VillageModel) -> Village {
        Village::try_from(model.clone()).expect("VillageModel to Village conversion must succeed")
    }

    fn split_army(army: &Army, requested: &TroopSet, carry_hero: bool) -> (Option<Army>, Army) {
        let mut returning = army.clone();
        returning.update_units(requested);
        if carry_hero {
            returning.set_hero(army.hero());
        } else {
            returning.set_hero(None);
        }

        let mut remaining_units = army.units().clone();
        for idx in 0..10 {
            remaining_units.remove(idx, requested.get(idx));
        }

        if remaining_units.immensity() == 0 && (!carry_hero || army.hero().is_none()) {
            (None, returning)
        } else {
            let mut remaining = army.clone();
            remaining.update_units(&remaining_units);
            if carry_hero {
                remaining.set_hero(None);
            }
            (Some(remaining), returning)
        }
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

    async fn deduct_village_resources_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
        cost: &ResourceGroup,
    ) -> Result<(), CqrsError> {
        if cost.total() == 0 {
            return Ok(());
        }
        let source = self
            .village
            .get_by_village_id_in_tx(tx, village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let next_resources = ResourceGroup::new(
            source.stocks.lumber.saturating_sub(cost.lumber()),
            source.stocks.clay.saturating_sub(cost.clay()),
            source.stocks.iron.saturating_sub(cost.iron()),
            (source.stocks.crop.max(0) as u32).saturating_sub(cost.crop()),
        );
        self.village
            .set_stored_resources_in_tx(tx, village_id, next_resources)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(())
    }

    async fn add_scheduled_action_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        action: &ScheduledAction,
    ) -> Result<(), CqrsError> {
        if !self.project_operational_actions {
            return Ok(());
        }
        self.actions
            .add_in_tx(tx, action)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    fn loyalty_regen_building_level(model: &VillageModel) -> u8 {
        model
            .buildings
            .iter()
            .filter(|b| {
                matches!(
                    b.building.name,
                    BuildingName::Residence | BuildingName::Palace
                )
            })
            .map(|b| b.building.level)
            .max()
            .unwrap_or(0)
    }

    fn loyalty_regen_interval(speed: i8) -> chrono::Duration {
        let speed = (speed as i64).max(1);
        chrono::Duration::seconds(((3 * 60 * 60) / speed).max(1))
    }

    async fn ensure_loyalty_regen_scheduled_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village: &VillageModel,
        now: chrono::DateTime<chrono::Utc>,
        exclude_action_id: Option<Uuid>,
    ) -> Result<(), CqrsError> {
        if !self.project_operational_actions || village.loyalty >= 100 {
            return Ok(());
        }
        let building_level = Self::loyalty_regen_building_level(village);
        if building_level == 0 {
            return Ok(());
        }
        let has_active: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM rm_scheduled_actions
                WHERE action_type = 'LoyaltyRegen'
                  AND (payload->>'village_id')::int = $1
                  AND status IN ('pending', 'processing')
                  AND ($2::uuid IS NULL OR id <> $2)
            )
            "#,
        )
        .bind(village.village_id as i32)
        .bind(exclude_action_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        if has_active {
            return Ok(());
        }

        let execute_at = now + Self::loyalty_regen_interval(parabellum_app::config::Config::from_env().speed);
        let action_id = Uuid::new_v4();
        let payload = ScheduledActionPayload::LoyaltyRegen {
            action_id,
            village_id: village.village_id,
            player_id: village.player_id,
            execute_at,
        };
        self.add_scheduled_action_in_tx(
            tx,
            &ScheduledAction {
                id: action_id,
                action_type: payload.action_type(),
                execute_at,
                payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                status: ScheduledActionStatus::Pending,
            },
        )
        .await
    }

    async fn set_stored_resources_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
        resources: ResourceGroup,
    ) -> Result<(), CqrsError> {
        self.village
            .set_stored_resources_in_tx(tx, village_id, resources)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn set_busy_merchants_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
        busy_merchants: u8,
    ) -> Result<(), CqrsError> {
        self.village
            .set_busy_merchants_in_tx(tx, village_id, busy_merchants)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn process_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &StoredEvent,
    ) -> Result<(), CqrsError> {
        if !event.aggregate_type.contains("VillageAggregate") {
            return Ok(());
        }

        let domain_event = event.get_payload::<VillageEvent>()?;
        match domain_event {
            VillageEvent::VillageArmyDetached {
                army: detached_army,
            } => {
                let village_id = event
                    .aggregate_id
                    .parse::<u32>()
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let current = self
                    .village
                    .get_by_village_id_in_tx(tx, village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let previous_home_army_id = current.army.as_ref().map(|a| a.id);
                let mut next_army = current.army;
                if let Some(ref mut army) = next_army {
                    let mut next_units = army.units().clone();
                    for idx in 0..10 {
                        next_units.remove(idx, detached_army.units().get(idx));
                    }
                    army.update_units(&next_units);
                    if detached_army.hero().is_some() {
                        army.set_hero(None);
                    }
                    if army.immensity() == 0 {
                        next_army = None;
                    }
                }
                self.village
                    .update_army_in_tx(tx, village_id, &next_army)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
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
                self.armies
                    .upsert_moving_in_tx(tx, &detached_army, village_id, current.player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if let Some(hero) = detached_army.hero() {
                    self.heroes
                        .upsert_in_tx(tx, &hero, hero.village_id, village_id, "moving")
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
                Ok(())
            }
            VillageEvent::HeroCreated {
                village_id, hero, ..
            } => self
                .heroes
                .upsert_in_tx(tx, &hero, village_id, village_id, "home")
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string())),
            VillageEvent::HeroRevivalScheduled {
                action_id,
                player_id,
                village_id,
                hero,
                reset,
                revive_at,
                ..
            } => {
                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
                        id: action_id,
                        action_type: ScheduledActionPayload::HeroRevival {
                            action_id,
                            village_id,
                            player_id,
                            hero: hero.clone(),
                            reset,
                            revive_at,
                        }
                        .action_type(),
                        execute_at: revive_at,
                        payload: serde_json::to_value(ScheduledActionPayload::HeroRevival {
                            action_id,
                            village_id,
                            player_id,
                            hero,
                            reset,
                            revive_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    },
                )
                .await
            }
            VillageEvent::HeroRevived {
                village_id, hero, ..
            } => self
                .heroes
                .upsert_in_tx(tx, &hero, village_id, village_id, "home")
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string())),
            VillageEvent::VillageFounded {
                village_id,
                village_name,
                position,
                tribe,
                player_id,
                parent_village_id,
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
                        parent_village_id,
                        &buildings,
                        &None,
                    )
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .set_map_occupancy_in_tx(tx, village_id, Some(village_id), Some(player_id))
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))
            }
            VillageEvent::VillageConquered {
                player_id,
                owner_village_id,
            } => {
                let village_id = event
                    .aggregate_id
                    .parse::<u32>()
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut conquered = self
                    .village
                    .get_by_village_id_in_tx(tx, village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                conquered.player_id = player_id;
                conquered.parent_village_id = Some(owner_village_id);
                conquered.loyalty = 100;
                self.village
                    .replace_village_state_in_tx(tx, &conquered)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .set_map_occupancy_in_tx(tx, village_id, Some(village_id), Some(player_id))
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))
            }
            VillageEvent::VillageResourcesSet {
                village_id,
                resources,
                ..
            } => {
                self.set_stored_resources_in_tx(tx, village_id, resources)
                    .await
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
                    .upsert_moving_in_tx(tx, &army, source_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if let Some(hero) = army.hero() {
                    self.heroes
                        .upsert_in_tx(tx, &hero, hero.village_id, source_village_id, "moving")
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
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
                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
                        id: movement_id,
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
                    },
                )
                .await
            }
            VillageEvent::ReinforcementArrived {
                movement_id,
                army_id,
                player_id: _,
                source_village_id,
                army,
                hero_alone_transfer,
                ..
            } => {
                let source = self
                    .village
                    .get_by_village_id_in_tx(tx, source_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if !hero_alone_transfer {
                    let mut source_deployed = source.deployed_armies;
                    source_deployed.push(army.clone());
                    self.village
                        .update_deployed_armies_in_tx(tx, source_village_id, &source_deployed)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
                self.movements
                    .delete_by_movement_id_in_tx(tx, movement_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .delete_in_tx(tx, army_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))
            }
            VillageEvent::ReinforcementAppliedToVillage {
                target_village_id,
                player_id,
                army,
                hero_alone_transfer,
                ..
            } => {
                let target = self
                    .village
                    .get_by_village_id_in_tx(tx, target_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if hero_alone_transfer {
                    let mut target_army = target.army.clone().unwrap_or_else(|| {
                        Army::new_village_army(&Self::village_from_model(&target))
                    });
                    target_army.set_hero(army.hero());
                    let next_target_army = if target_army.immensity() == 0 {
                        None
                    } else {
                        Some(target_army)
                    };
                    self.village
                        .update_army_in_tx(tx, target_village_id, &next_target_army)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    if let Some(ref home_army) = next_target_army {
                        self.armies
                            .upsert_home_in_tx(tx, home_army, target.player_id)
                            .await
                            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    }
                    if let Some(mut hero) = army.hero() {
                        hero.village_id = target_village_id;
                        self.heroes
                            .upsert_in_tx(tx, &hero, target_village_id, target_village_id, "home")
                            .await
                            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    }
                } else {
                    let mut target_reinforcements = target.reinforcements;
                    target_reinforcements.push(army.clone());
                    self.village
                        .update_reinforcements_in_tx(tx, target_village_id, &target_reinforcements)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    self.armies
                        .upsert_stationed_in_tx(tx, &army, target_village_id, player_id)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    if let Some(hero) = army.hero() {
                        self.heroes
                            .upsert_in_tx(
                                tx,
                                &hero,
                                hero.village_id,
                                target_village_id,
                                "stationed",
                            )
                            .await
                            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    }
                }
                Ok(())
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
            } => {
                let source = self
                    .village
                    .get_by_village_id_in_tx(tx, home_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next_deployed = source.deployed_armies;
                if let Some(idx) = next_deployed.iter().position(|a| a.id == army_id) {
                    let existing = next_deployed[idx].clone();
                    let (remaining, _) =
                        Self::split_army(&existing, army.units(), army.hero().is_some());
                    if let Some(remaining) = remaining {
                        next_deployed[idx] = remaining;
                    } else {
                        next_deployed.remove(idx);
                    }
                    self.village
                        .update_deployed_armies_in_tx(tx, home_village_id, &next_deployed)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }

                let stationed = self
                    .village
                    .get_by_village_id_in_tx(tx, stationed_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next_reinforcements = stationed.reinforcements;
                if let Some(idx) = next_reinforcements.iter().position(|a| a.id == army_id) {
                    let existing = next_reinforcements[idx].clone();
                    let (remaining, _) =
                        Self::split_army(&existing, army.units(), army.hero().is_some());
                    if let Some(remaining) = remaining {
                        next_reinforcements[idx] = remaining.clone();
                        self.armies
                            .upsert_stationed_in_tx(
                                tx,
                                &remaining,
                                stationed_village_id,
                                remaining.player_id,
                            )
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
                        next_reinforcements.remove(idx);
                        self.armies
                            .delete_in_tx(tx, army_id)
                            .await
                            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    }
                    self.village
                        .update_reinforcements_in_tx(tx, stationed_village_id, &next_reinforcements)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }

                let outgoing = VillageMovement {
                    movement_id,
                    movement_type: MovementType::Return,
                    direction: MovementDirection::Outgoing,
                    origin_village_id: stationed_village_id,
                    origin_village_name: None,
                    origin_player_id: army.player_id,
                    origin_position: None,
                    target_village_id: home_village_id,
                    target_village_name: None,
                    target_player_id: None,
                    target_position: None,
                    arrives_at: returns_at,
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
                self.armies
                    .upsert_moving_in_tx(tx, &army, stationed_village_id, army.player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if let Some(hero) = army.hero() {
                    self.heroes
                        .upsert_in_tx(tx, &hero, hero.village_id, stationed_village_id, "moving")
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }

                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
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
                    },
                )
                .await
            }
            VillageEvent::ReinforcementsReleased {
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
                    .get_by_village_id_in_tx(tx, home_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next_deployed = source.deployed_armies;
                if let Some(idx) = next_deployed.iter().position(|a| a.id == army_id) {
                    let existing = next_deployed[idx].clone();
                    let (remaining, _) =
                        Self::split_army(&existing, army.units(), army.hero().is_some());
                    if let Some(remaining) = remaining {
                        next_deployed[idx] = remaining;
                    } else {
                        next_deployed.remove(idx);
                    }
                    self.village
                        .update_deployed_armies_in_tx(tx, home_village_id, &next_deployed)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }

                let stationed = self
                    .village
                    .get_by_village_id_in_tx(tx, stationed_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next_reinforcements = stationed.reinforcements;
                if let Some(idx) = next_reinforcements.iter().position(|a| a.id == army_id) {
                    let existing = next_reinforcements[idx].clone();
                    let (remaining, _) =
                        Self::split_army(&existing, army.units(), army.hero().is_some());
                    if let Some(remaining) = remaining {
                        next_reinforcements[idx] = remaining.clone();
                        self.armies
                            .upsert_stationed_in_tx(
                                tx,
                                &remaining,
                                stationed_village_id,
                                remaining.player_id,
                            )
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
                        next_reinforcements.remove(idx);
                        self.armies
                            .delete_in_tx(tx, army_id)
                            .await
                            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    }
                    self.village
                        .update_reinforcements_in_tx(tx, stationed_village_id, &next_reinforcements)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }

                // Released reinforcements are only visible to the reinforcement owner (home side).
                let incoming = VillageMovement {
                    movement_id,
                    movement_type: MovementType::Return,
                    direction: MovementDirection::Incoming,
                    origin_village_id: stationed_village_id,
                    origin_village_name: None,
                    origin_player_id: army.player_id,
                    origin_position: None,
                    target_village_id: home_village_id,
                    target_village_name: None,
                    target_player_id: None,
                    target_position: None,
                    arrives_at: returns_at,
                    time_seconds: None,
                    units: army.units().clone(),
                    tribe: Some(army.tribe.clone()),
                    bounty: None,
                };
                self.movements
                    .upsert_in_tx(tx, &incoming)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .upsert_moving_in_tx(tx, &army, stationed_village_id, army.player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if let Some(hero) = army.hero() {
                    self.heroes
                        .upsert_in_tx(tx, &hero, hero.village_id, stationed_village_id, "moving")
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }

                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
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
                    },
                )
                .await
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
                    .upsert_moving_in_tx(tx, &army, source_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if let Some(hero) = army.hero() {
                    self.heroes
                        .upsert_in_tx(tx, &hero, hero.village_id, source_village_id, "moving")
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
                let outgoing = VillageMovement {
                    movement_id,
                    movement_type: MovementType::FoundVillage,
                    direction: MovementDirection::Outgoing,
                    origin_village_id: source_village_id,
                    origin_village_name: None,
                    origin_player_id: player_id,
                    origin_position: None,
                    // `rm_village_movements.target_village_id` is FK-backed and an
                    // unoccupied valley has no rm_village row yet. Keep FK valid
                    // and carry real destination in `target_position`.
                    target_village_id: source_village_id,
                    target_village_name: Some(village_name.clone()),
                    target_player_id: Some(player_id),
                    target_position: Some(target_position.clone()),
                    arrives_at,
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
                    .get_by_village_id_in_tx(tx, source_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.set_stored_resources_in_tx(
                    tx,
                    source_village_id,
                    ResourceGroup::new(
                        source.stocks.lumber.saturating_sub(800),
                        source.stocks.clay.saturating_sub(800),
                        source.stocks.iron.saturating_sub(800),
                        source.stocks.crop.saturating_sub(800).max(0) as u32,
                    ),
                )
                .await?;

                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
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
                    },
                )
                .await
            }
            VillageEvent::SettlersArrived {
                movement_id, army_id, ..
            } => {
                self.movements
                    .delete_by_movement_id_in_tx(tx, movement_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .delete_in_tx(tx, army_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))
            }
            VillageEvent::AttackSent {
                movement_id,
                player_id,
                source_village_id,
                target_village_id,
                army,
                attack_type,
                arrives_at,
                ..
            } => {
                self.armies
                    .upsert_moving_in_tx(tx, &army, source_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if let Some(hero) = army.hero() {
                    self.heroes
                        .upsert_in_tx(tx, &hero, hero.village_id, source_village_id, "moving")
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
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
            VillageEvent::AttackArrivalScheduled {
                action_id,
                movement_id,
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
                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
                        id: action_id,
                        action_type: ScheduledActionPayload::AttackArrival {
                            action_id,
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
                            action_id,
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
                    },
                )
                .await
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
                    .upsert_moving_in_tx(tx, &army, source_village_id, player_id)
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

                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
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
                    },
                )
                .await
            }
            VillageEvent::AttackArrived {
                movement_id,
                army_id,
                ..
            } => {
                self.movements
                    .delete_by_movement_id_in_tx(tx, movement_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .delete_in_tx(tx, army_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))
            }
            VillageEvent::ScoutArrived {
                movement_id,
                army_id,
                ..
            } => {
                self.movements
                    .delete_by_movement_id_in_tx(tx, movement_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .delete_in_tx(tx, army_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))
            }
            VillageEvent::AttackBattleResolved {
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
            } => {
                let source = self
                    .village
                    .get_by_village_id_in_tx(tx, source_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                if let Some(stationed_attacker) = stationed_attacker_army.clone() {
                    let mut source_deployed = source.deployed_armies;
                    source_deployed.push(stationed_attacker);
                    self.village
                        .update_deployed_armies_in_tx(tx, source_village_id, &source_deployed)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }

                let Some(return_army) = returning_army else {
                    return Ok(());
                };
                let outgoing = VillageMovement {
                    movement_id,
                    movement_type: MovementType::Return,
                    direction: MovementDirection::Outgoing,
                    origin_village_id: target_village_id,
                    origin_village_name: None,
                    origin_player_id: player_id,
                    origin_position: None,
                    target_village_id: source_village_id,
                    target_village_name: None,
                    target_player_id: None,
                    target_position: None,
                    arrives_at: returns_at,
                    time_seconds: None,
                    units: return_army.units().clone(),
                    tribe: Some(return_army.tribe.clone()),
                    bounty: report.bounty.clone(),
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
                self.armies
                    .upsert_moving_in_tx(tx, &return_army, target_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if let Some(hero) = return_army.hero() {
                    self.heroes
                        .upsert_in_tx(tx, &hero, hero.village_id, target_village_id, "moving")
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
                        id: return_action_id,
                        action_type: ScheduledActionPayload::ArmyReturn {
                            action_id: return_action_id,
                            movement_id,
                            army_id: return_army.id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            army: return_army.clone(),
                            bounty: report.bounty.clone(),
                            returns_at,
                        }
                        .action_type(),
                        execute_at: returns_at,
                        payload: serde_json::to_value(ScheduledActionPayload::ArmyReturn {
                            action_id: return_action_id,
                            movement_id,
                            army_id: return_army.id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            army: return_army,
                            bounty: report.bounty,
                            returns_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    },
                )
                .await
            }
            VillageEvent::BattleOutcomeAppliedToVillage {
                source_village_id,
                target_village_id,
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
            } => {
                let target_before = self
                    .village
                    .get_by_village_id_in_tx(tx, target_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let mut target_next = target_before.clone();
                target_next.player_id = target_player_id;
                target_next.parent_village_id = target_parent_village_id;
                target_next.loyalty = target_loyalty;
                target_next.buildings = target_buildings;
                target_next.production = target_production;
                target_next.population = target_population;
                target_next.stocks = target_stocks;
                target_next.army = target_army;
                target_next.reinforcements = target_reinforcements;
                if let Some(stationed_attacker) = stationed_attacker_army {
                    target_next.reinforcements.push(stationed_attacker.clone());
                }

                let mut before_ids: HashSet<Uuid> = HashSet::new();
                if let Some(before_home) = target_before.army.as_ref() {
                    before_ids.insert(before_home.id);
                }
                for before_reinforcement in &target_before.reinforcements {
                    before_ids.insert(before_reinforcement.id);
                }

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
                            .upsert_in_tx(
                                tx,
                                &hero,
                                hero.village_id,
                                target_village_id,
                                "stationed",
                            )
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

                if target_player_id != target_before.player_id {
                    let source = self
                        .village
                        .get_by_village_id_in_tx(tx, source_village_id)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    target_next.tribe = source.tribe.clone();
                    let mut conquered_village = Self::village_from_model(&target_next);
                    conquered_village.tribe = source.tribe;
                    Self::remove_tribe_incompatible_buildings(&mut conquered_village);
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
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }

                self.village
                    .replace_village_state_in_tx(tx, &target_next)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.ensure_loyalty_regen_scheduled_in_tx(
                    tx,
                    &target_next,
                    chrono::Utc::now(),
                    None,
                )
                    .await?;

                let mut before_by_home: std::collections::HashMap<u32, Vec<uuid::Uuid>> =
                    std::collections::HashMap::new();
                for reinforcement in &target_before.reinforcements {
                    before_by_home
                        .entry(reinforcement.village_id)
                        .or_default()
                        .push(reinforcement.id);
                }
                let mut after_by_home: std::collections::HashMap<u32, Vec<Army>> =
                    std::collections::HashMap::new();
                for reinforcement in &target_next.reinforcements {
                    after_by_home
                        .entry(reinforcement.village_id)
                        .or_default()
                        .push(reinforcement.clone());
                }
                let mut homes = std::collections::BTreeSet::new();
                homes.extend(before_by_home.keys().copied());
                homes.extend(after_by_home.keys().copied());
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
                        if let Some(pos) = next_deployed.iter().position(|army| army.id == removed)
                        {
                            next_deployed.remove(pos);
                        }
                    }
                    let after_armies = after_by_home
                        .get(&home_village_id)
                        .cloned()
                        .unwrap_or_default();
                    for updated in after_armies {
                        if let Some(pos) =
                            next_deployed.iter().position(|army| army.id == updated.id)
                        {
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
            VillageEvent::ScoutBattleResolved {
                movement_id,
                return_action_id,
                player_id,
                source_village_id,
                target_village_id,
                report: _,
                returning_army,
                returns_at,
                ..
            } => {
                let Some(return_army) = returning_army else {
                    return Ok(());
                };
                let outgoing = VillageMovement {
                    movement_id,
                    movement_type: MovementType::Return,
                    direction: MovementDirection::Outgoing,
                    origin_village_id: target_village_id,
                    origin_village_name: None,
                    origin_player_id: player_id,
                    origin_position: None,
                    target_village_id: source_village_id,
                    target_village_name: None,
                    target_player_id: None,
                    target_position: None,
                    arrives_at: returns_at,
                    time_seconds: None,
                    units: return_army.units().clone(),
                    tribe: Some(return_army.tribe.clone()),
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
                self.armies
                    .upsert_moving_in_tx(tx, &return_army, target_village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if let Some(hero) = return_army.hero() {
                    self.heroes
                        .upsert_in_tx(tx, &hero, hero.village_id, target_village_id, "moving")
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
                        id: return_action_id,
                        action_type: ScheduledActionPayload::ArmyReturn {
                            action_id: return_action_id,
                            movement_id,
                            army_id: return_army.id,
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
                            army_id: return_army.id,
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
                    },
                )
                .await
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
                    .get_by_village_id_in_tx(tx, source_village_id)
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
                if source_army.hero().is_none() {
                    source_army.set_hero(army.hero());
                }
                source_deployed.retain(|army| army.id != movement_id);
                let next_source_army = if source_army.immensity() == 0 {
                    None
                } else {
                    Some(source_army)
                };
                self.village
                    .update_army_in_tx(tx, source_village_id, &next_source_army)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .update_deployed_armies_in_tx(tx, source_village_id, &source_deployed)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.movements
                    .delete_by_movement_id_in_tx(tx, movement_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .delete_in_tx(tx, movement_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.armies
                    .delete_in_tx(tx, army.id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if let Some(ref home_army) = next_source_army {
                    self.armies
                        .upsert_home_in_tx(tx, home_army, player_id)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    if let Some(hero) = home_army.hero() {
                        self.heroes
                            .upsert_in_tx(tx, &hero, source_village_id, source_village_id, "home")
                            .await
                            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    }
                }

                if let Some(bounty) = bounty {
                    let next_resources = ResourceGroup::new(
                        source.stocks.lumber.saturating_add(bounty.lumber()),
                        source.stocks.clay.saturating_add(bounty.clay()),
                        source.stocks.iron.saturating_add(bounty.iron()),
                        (source.stocks.crop.max(0) as u32).saturating_add(bounty.crop()),
                    );
                    self.set_stored_resources_in_tx(tx, source_village_id, next_resources)
                        .await?;
                }
                Ok(())
            }
            VillageEvent::BuildingConstructionScheduled {
                action_id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                cost,
                execute_at,
            } => {
                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
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
                            building_name: building_name.clone(),
                            level,
                            speed,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    },
                )
                .await?;
                self.deduct_village_resources_in_tx(tx, village_id, &cost)
                    .await
            }
            VillageEvent::BuildingUpgradeScheduled {
                action_id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                cost,
                execute_at,
            } => {
                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
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
                            building_name: building_name.clone(),
                            level,
                            speed,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    },
                )
                .await?;
                self.deduct_village_resources_in_tx(tx, village_id, &cost)
                    .await
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
                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
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
                    },
                )
                .await
            }
            VillageEvent::UnitTrainingScheduled {
                action_id,
                player_id,
                village_id,
                slot_id,
                unit,
                time_per_unit,
                quantity_remaining,
                cost,
                execute_at,
            } => {
                let payload = ScheduledActionPayload::TrainUnit {
                    action_id,
                    village_id,
                    player_id,
                    slot_id,
                    unit: unit.clone(),
                    time_per_unit,
                    quantity_remaining,
                    execute_at,
                };
                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
                        id: action_id,
                        action_type: payload.action_type(),
                        execute_at,
                        payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    },
                )
                .await?;
                self.deduct_village_resources_in_tx(tx, village_id, &cost)
                    .await
            }
            VillageEvent::UnitTrained {
                village_id,
                unit,
                quantity_trained,
                ..
            } => {
                let current = self
                    .village
                    .get_by_village_id_in_tx(tx, village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next_army = current
                    .army
                    .clone()
                    .unwrap_or_else(|| Army::new_village_army(&Self::village_from_model(&current)));
                next_army
                    .add_unit(unit, quantity_trained)
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .update_army_in_tx(tx, village_id, &Some(next_army))
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let refreshed = self
                    .village
                    .get_by_village_id_in_tx(tx, village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                if let Some(army) = &refreshed.army {
                    self.armies
                        .upsert_home_in_tx(tx, army, refreshed.player_id)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
                Ok(())
            }
            VillageEvent::AcademyResearchScheduled {
                action_id,
                player_id,
                village_id,
                unit,
                cost,
                execute_at,
            } => {
                let payload = ScheduledActionPayload::ResearchAcademy {
                    action_id,
                    village_id,
                    player_id,
                    unit: unit.clone(),
                };
                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
                        id: action_id,
                        action_type: payload.action_type(),
                        execute_at,
                        payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    },
                )
                .await?;
                self.deduct_village_resources_in_tx(tx, village_id, &cost)
                    .await
            }
            VillageEvent::AcademyResearchCompleted {
                village_id, unit, ..
            } => {
                let current = self
                    .village
                    .get_by_village_id_in_tx(tx, village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut village = Self::village_from_model(&current);
                village
                    .research_academy(unit)
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next = current.clone();
                next.academy_research = village.academy_research().clone();
                self.village
                    .replace_village_state_in_tx(tx, &next)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))
            }
            VillageEvent::SmithyResearchScheduled {
                action_id,
                player_id,
                village_id,
                unit,
                cost,
                execute_at,
            } => {
                let payload = ScheduledActionPayload::ResearchSmithy {
                    action_id,
                    village_id,
                    player_id,
                    unit: unit.clone(),
                };
                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
                        id: action_id,
                        action_type: payload.action_type(),
                        execute_at,
                        payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    },
                )
                .await?;
                self.deduct_village_resources_in_tx(tx, village_id, &cost)
                    .await
            }
            VillageEvent::SmithyResearchCompleted {
                village_id, unit, ..
            } => {
                let current = self
                    .village
                    .get_by_village_id_in_tx(tx, village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut village = Self::village_from_model(&current);
                village
                    .upgrade_smithy(unit)
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next = current.clone();
                next.smithy_upgrades = *village.smithy();
                self.village
                    .replace_village_state_in_tx(tx, &next)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))
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
                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
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
                    },
                )
                .await?;

                self.add_scheduled_action_in_tx(
                    tx,
                    &ScheduledAction {
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
                    },
                )
                .await?;

                if !resources_already_reserved {
                    let source = self
                        .village
                        .get_by_village_id_in_tx(tx, source_village_id)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    let next_resources = ResourceGroup::new(
                        source.stocks.lumber.saturating_sub(resources.lumber()),
                        source.stocks.clay.saturating_sub(resources.clay()),
                        source.stocks.iron.saturating_sub(resources.iron()),
                        (source.stocks.crop.max(0) as u32).saturating_sub(resources.crop()),
                    );
                    self.set_stored_resources_in_tx(tx, source_village_id, next_resources)
                        .await?;
                    let next_busy = source.busy_merchants.saturating_add(merchants_used);
                    self.set_busy_merchants_in_tx(tx, source_village_id, next_busy)
                        .await?;
                }
                Ok(())
            }
            VillageEvent::MerchantTransferAppliedToVillage {
                target_village_id,
                target_stocks,
                ..
            } => {
                let next_resources = ResourceGroup::new(
                    target_stocks.lumber,
                    target_stocks.clay,
                    target_stocks.iron,
                    target_stocks.crop.max(0) as u32,
                );
                self.set_stored_resources_in_tx(tx, target_village_id, next_resources)
                    .await
            }
            VillageEvent::MerchantsReturned {
                source_village_id,
                merchants_used,
                ..
            } => {
                let source = self
                    .village
                    .get_by_village_id_in_tx(tx, source_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let next_busy = source.busy_merchants.saturating_sub(merchants_used);
                self.set_busy_merchants_in_tx(tx, source_village_id, next_busy)
                    .await
            }
            VillageEvent::MarketplaceOfferCreated {
                offer_id,
                owner_player_id,
                owner_village_id,
                offer_resources,
                seek_resources,
                merchants_reserved,
                created_at,
            } => self
                .offers
                .upsert_in_tx(
                    tx,
                    &parabellum_app::villages::models::MarketplaceOfferModel {
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
                    },
                )
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string())),
            VillageEvent::MarketplaceOfferReservationAppliedToVillage {
                owner_village_id,
                owner_stocks,
                owner_busy_merchants,
                ..
            } => {
                let next_resources = ResourceGroup::new(
                    owner_stocks.lumber,
                    owner_stocks.clay,
                    owner_stocks.iron,
                    owner_stocks.crop.max(0) as u32,
                );
                self.set_stored_resources_in_tx(tx, owner_village_id, next_resources)
                    .await?;
                self.set_busy_merchants_in_tx(tx, owner_village_id, owner_busy_merchants)
                    .await
            }
            VillageEvent::MarketplaceOfferCanceled {
                offer_id,
                canceled_at,
                ..
            } => self
                .offers
                .set_status_in_tx(
                    tx,
                    offer_id,
                    parabellum_app::villages::models::MarketplaceOfferStatus::Canceled,
                    None,
                    None,
                    canceled_at,
                )
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string())),
            VillageEvent::MarketplaceOfferReservationReleasedFromVillage {
                owner_village_id,
                owner_stocks,
                owner_busy_merchants,
                ..
            } => {
                let next_resources = ResourceGroup::new(
                    owner_stocks.lumber,
                    owner_stocks.clay,
                    owner_stocks.iron,
                    owner_stocks.crop.max(0) as u32,
                );
                self.set_stored_resources_in_tx(tx, owner_village_id, next_resources)
                    .await?;
                self.set_busy_merchants_in_tx(tx, owner_village_id, owner_busy_merchants)
                    .await
            }
            VillageEvent::MarketplaceOfferAccepted {
                offer_id,
                accepting_player_id,
                accepting_village_id,
                accepted_at,
                ..
            } => self
                .offers
                .set_status_in_tx(
                    tx,
                    offer_id,
                    parabellum_app::villages::models::MarketplaceOfferStatus::Accepted,
                    Some(accepting_player_id),
                    Some(accepting_village_id),
                    accepted_at,
                )
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string())),
            VillageEvent::MarketplaceOfferAcceptanceAppliedToVillage {
                village_id,
                stocks,
                busy_merchants,
                ..
            } => {
                let next_resources = ResourceGroup::new(
                    stocks.lumber,
                    stocks.clay,
                    stocks.iron,
                    stocks.crop.max(0) as u32,
                );
                self.set_stored_resources_in_tx(tx, village_id, next_resources)
                    .await?;
                self.set_busy_merchants_in_tx(tx, village_id, busy_merchants)
                    .await
            }
            VillageEvent::MerchantsArrived { .. } => Ok(()),
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
                    .update_building_in_tx(tx, village_id, slot_id, building_name, level, speed)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let village = self
                    .village
                    .get_by_village_id_in_tx(tx, village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.ensure_loyalty_regen_scheduled_in_tx(
                    tx,
                    &village,
                    chrono::Utc::now(),
                    None,
                )
                    .await
            }
            VillageEvent::LoyaltyRegenerated {
                action_id,
                village_id,
                loyalty_after,
                ..
            } => {
                let mut village = self
                    .village
                    .get_by_village_id_in_tx(tx, village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                village.loyalty = loyalty_after;
                self.village
                    .replace_village_state_in_tx(tx, &village)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.ensure_loyalty_regen_scheduled_in_tx(
                    tx,
                    &village,
                    chrono::Utc::now(),
                    Some(action_id),
                )
                    .await
            }
            VillageEvent::ReportMarkedAsRead { .. } => Ok(()),
        }
    }
}

impl EventConsumer for VillageProjector {
    async fn process(&self, event: &StoredEvent) -> Result<(), CqrsError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.process_in_tx(&mut tx, event).await?;
        tx.commit()
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(())
    }
}
