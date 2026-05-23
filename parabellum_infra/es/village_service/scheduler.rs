//! Scheduled-action payload executor for `VillageEsService`.
//!
//! Each payload variant is executed as deterministic workflow progression.
//! Validation is assumed to have happened at scheduling time; this layer executes
//! payload intent and applies terminal status (`completed`/`failed`) upstream.
//!
//! Workflow fact builders:
//! - Keep branch logic in `execute_action` thin by delegating payload-to-fact
//!   construction to focused `build_*_fact(s)` helpers.
//! - Use pure builders for deterministic outcomes that do not require I/O.
//! - Use async builders when outcome production needs read-side/domain lookups
//!   (e.g. battle/scout/merchant arrival computations).
//! - New scheduled workflows should follow the same shape: validate -> build
//!   canonical fact(s) -> append via `append_village_workflow_events`.

use super::*;
use parabellum_app::villages::VillageEvent;
use parabellum_game::battle::Battle;
use parabellum_game::models::army::Army;
use parabellum_game::models::buildings::Building;
use parabellum_game::models::map::MapFieldTopology;
use parabellum_game::models::smithy::SmithyUpgrades;
use parabellum_game::models::village::{Village, VillageBuilding};
use parabellum_types::army::UnitRole;
use parabellum_types::buildings::BuildingName;
use parabellum_types::map::ValleyTopology;

struct ComputedAttackOutcome {
    source: ResolveAttackBattle,
    target: ApplyBattleOutcomeToVillage,
}

struct ComputedScoutOutcome {
    fact: VillageEvent,
}

const DEFAULT_FOUNDATION_SPEED: i8 = 1;

fn default_founded_village_buildings(
    topology: &ValleyTopology,
    speed: i8,
) -> Result<Vec<VillageBuilding>, CqrsError> {
    let mut slot_id: u8 = 1;
    let mut buildings = Vec::with_capacity(19);

    let mut push_n = |name: BuildingName, count: u8| -> Result<(), CqrsError> {
        for _ in 0..count {
            let building = Building::new(name.clone(), speed)
                .at_level(0, speed)
                .map_err(CqrsError::domain_source)?;
            buildings.push(VillageBuilding { slot_id, building });
            slot_id += 1;
        }
        Ok(())
    };

    push_n(BuildingName::Woodcutter, topology.lumber())?;
    push_n(BuildingName::ClayPit, topology.clay())?;
    push_n(BuildingName::IronMine, topology.iron())?;
    push_n(BuildingName::Cropland, topology.crop())?;

    let main_building = Building::new(BuildingName::MainBuilding, speed)
        .at_level(1, speed)
        .map_err(CqrsError::domain_source)?;
    buildings.push(VillageBuilding {
        slot_id: 19,
        building: main_building,
    });

    Ok(buildings)
}

fn build_army_return_fact(
    action_id: uuid::Uuid,
    movement_id: uuid::Uuid,
    army_id: uuid::Uuid,
    player_id: uuid::Uuid,
    source_village_id: u32,
    target_village_id: u32,
    army: Army,
    bounty: Option<parabellum_types::common::ResourceGroup>,
    returns_at: chrono::DateTime<chrono::Utc>,
) -> VillageEvent {
    VillageEvent::ArmyReturned {
        action_id,
        movement_id,
        army_id,
        player_id,
        source_village_id,
        target_village_id,
        army,
        bounty,
        returns_at,
    }
}

fn build_merchant_return_fact(
    action_id: uuid::Uuid,
    player_id: uuid::Uuid,
    source_village_id: u32,
    merchants_used: u8,
    returns_at: chrono::DateTime<chrono::Utc>,
) -> VillageEvent {
    VillageEvent::MerchantsReturned {
        action_id,
        player_id,
        source_village_id,
        merchants_used,
        returns_at,
    }
}

fn build_building_added_fact(
    action_id: uuid::Uuid,
    player_id: uuid::Uuid,
    village_id: u32,
    slot_id: u8,
    building_name: parabellum_types::buildings::BuildingName,
    level: u8,
    speed: i8,
) -> VillageEvent {
    VillageEvent::BuildingAdded {
        action_id,
        player_id,
        village_id,
        slot_id,
        building_name,
        level,
        speed,
    }
}

fn build_building_upgraded_fact(
    action_id: uuid::Uuid,
    player_id: uuid::Uuid,
    village_id: u32,
    slot_id: u8,
    building_name: parabellum_types::buildings::BuildingName,
    level: u8,
    speed: i8,
) -> VillageEvent {
    VillageEvent::BuildingUpgraded {
        action_id,
        player_id,
        village_id,
        slot_id,
        building_name,
        level,
        speed,
    }
}

fn build_building_downgraded_fact(
    action_id: uuid::Uuid,
    player_id: uuid::Uuid,
    village_id: u32,
    slot_id: u8,
    building_name: parabellum_types::buildings::BuildingName,
    level: u8,
    speed: i8,
) -> VillageEvent {
    VillageEvent::BuildingDowngraded {
        action_id,
        player_id,
        village_id,
        slot_id,
        building_name,
        level,
        speed,
    }
}

fn build_train_unit_completion_facts(
    action_id: uuid::Uuid,
    player_id: uuid::Uuid,
    village_id: u32,
    slot_id: u8,
    unit: parabellum_types::army::UnitName,
    time_per_unit: i32,
    quantity_remaining: i32,
    execute_at: chrono::DateTime<chrono::Utc>,
) -> Vec<(u32, VillageEvent)> {
    if quantity_remaining <= 0 {
        return vec![];
    }
    let mut workflow_events = vec![(
        village_id,
        VillageEvent::UnitTrained {
            action_id,
            player_id,
            village_id,
            unit: unit.clone(),
            quantity_trained: 1,
        },
    )];
    let remaining_after = quantity_remaining - 1;
    if remaining_after > 0 {
        workflow_events.push((
            village_id,
            VillageEvent::UnitTrainingScheduled {
                action_id: uuid::Uuid::new_v4(),
                player_id,
                village_id,
                slot_id,
                unit,
                time_per_unit,
                quantity_remaining: remaining_after,
                cost: parabellum_types::common::ResourceGroup::new(0, 0, 0, 0),
                execute_at: execute_at + chrono::Duration::seconds(time_per_unit.max(1) as i64),
            },
        ));
    }
    workflow_events
}

fn build_academy_research_completed_fact(
    action_id: uuid::Uuid,
    player_id: uuid::Uuid,
    village_id: u32,
    unit: parabellum_types::army::UnitName,
) -> VillageEvent {
    VillageEvent::AcademyResearchCompleted {
        action_id,
        player_id,
        village_id,
        unit,
    }
}

fn build_smithy_research_completed_fact(
    action_id: uuid::Uuid,
    player_id: uuid::Uuid,
    village_id: u32,
    unit: parabellum_types::army::UnitName,
) -> VillageEvent {
    VillageEvent::SmithyResearchCompleted {
        action_id,
        player_id,
        village_id,
        unit,
    }
}

/// Executes one scheduled action payload by appending canonical workflow fact(s).
pub(super) async fn execute_action(
    svc: &VillageEsService,
    _service: &VillageService<'_, crate::es::VillageCqrsRuntime>,
    action: &parabellum_app::villages::models::ScheduledAction,
) -> Result<(), CqrsError> {
    tracing::debug!(
        action_id = %action.id,
        execute_at = %action.execute_at,
        action_type = ?action.action_type,
        "executing scheduled action"
    );
    let payload: ScheduledActionPayload =
        serde_json::from_value(action.payload.clone()).map_err(CqrsError::Serialization)?;
    match payload {
        ScheduledActionPayload::ReinforcementArrival {
            movement_id,
            army_id,
            player_id,
            source_village_id,
            target_village_id,
            army,
            arrives_at,
        } => {
            let source = svc.get_village(source_village_id).await?;
            let target = svc.get_village(target_village_id).await?;
            let hero_alone_transfer = army.hero().is_some()
                && army.units().immensity() == 0
                && source.player_id == target.player_id
                && source.buildings.iter().any(|b| {
                    b.building.name == parabellum_types::buildings::BuildingName::HeroMansion
                        && b.building.level > 0
                })
                && target.buildings.iter().any(|b| {
                    b.building.name == parabellum_types::buildings::BuildingName::HeroMansion
                        && b.building.level > 0
                });
            svc.append_village_workflow_events(vec![
                (
                    source_village_id,
                    VillageEvent::ReinforcementArrived {
                        movement_id,
                        army_id,
                        player_id,
                        source_village_id,
                        target_village_id,
                        army: army.clone(),
                        hero_alone_transfer,
                        arrives_at,
                    },
                ),
                (
                    target_village_id,
                    VillageEvent::ReinforcementAppliedToVillage {
                        movement_id,
                        army_id,
                        player_id,
                        source_village_id,
                        target_village_id,
                        army,
                        hero_alone_transfer,
                        arrives_at,
                    },
                ),
            ])
            .await?;
        }
        ScheduledActionPayload::SettlersArrival {
            action_id,
            movement_id,
            army_id,
            village_id: _,
            source_village_id,
            target_village_id,
            target_position,
            player_id,
            village_name,
            tribe,
            arrives_at,
        } => {
            let field_exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM rm_map_fields WHERE id = $1)")
                    .bind(target_village_id as i32)
                    .fetch_one(&svc.pool)
                    .await
                    .map_err(CqrsError::domain_source)?;
            let can_found = if field_exists {
                let claim = sqlx::query_scalar::<_, i64>(
                    r#"
                        SELECT COUNT(*)::bigint
                        FROM rm_map_fields
                        WHERE id = $1
                          AND village_id IS NULL
                          AND (player_id IS NULL OR player_id = $2)
                        "#,
                )
                .bind(target_village_id as i32)
                .bind(player_id)
                .fetch_one(&svc.pool)
                .await
                .map_err(CqrsError::domain_source)?;
                claim > 0
            } else {
                true
            };

            if can_found {
                let target_topology_json: Option<serde_json::Value> =
                    sqlx::query_scalar("SELECT topology FROM rm_map_fields WHERE id = $1")
                        .bind(target_village_id as i32)
                        .fetch_optional(&svc.pool)
                        .await
                        .map_err(CqrsError::domain_source)?;
                let target_topology = target_topology_json
                    .and_then(|value| serde_json::from_value::<MapFieldTopology>(value).ok());
                let topology = match target_topology {
                    Some(MapFieldTopology::Valley(valley)) => valley,
                    _ => ValleyTopology(4, 4, 4, 6),
                };
                let default_buildings =
                    default_founded_village_buildings(&topology, DEFAULT_FOUNDATION_SPEED)?;

                svc.append_village_workflow_events(vec![
                    (
                        source_village_id,
                        VillageEvent::SettlersArrived {
                            action_id,
                            movement_id,
                            army_id,
                            player_id,
                            source_village_id,
                            target_village_id,
                            target_position: target_position.clone(),
                            village_name: village_name.clone(),
                            tribe: tribe.clone(),
                            arrives_at,
                        },
                    ),
                    (
                        target_village_id,
                        VillageEvent::VillageFounded {
                            village_id: target_village_id,
                            village_name,
                            position: target_position,
                            tribe,
                            player_id,
                            parent_village_id: Some(source_village_id),
                            buildings: default_buildings,
                        },
                    ),
                ])
                .await?;
            } else {
                tracing::warn!(
                    action_id = %action_id,
                    player_id = %player_id,
                    source_village_id,
                    target_village_id,
                    "settlers arrival target unavailable, scheduling army return"
                );
                let army_repo: Arc<dyn ArmyRepository> =
                    Arc::new(PostgresArmyRepository::new(svc.pool.clone()));
                let army = army_repo
                    .get_moving_army(army_id)
                    .await
                    .map_err(CqrsError::domain_source)?;
                let source = svc.get_village(source_village_id).await?;
                let cfg = parabellum_app::config::Config::from_env();
                let travel_secs = source.position.calculate_travel_time_secs(
                    target_position.clone(),
                    army.speed(),
                    cfg.world_size as i32,
                    cfg.speed as u8,
                ) as i64;
                let returns_at =
                    arrives_at + chrono::Duration::seconds(std::cmp::max(1, travel_secs));
                let return_action_id = uuid::Uuid::new_v4();
                PostgresScheduledActionRepository::new(svc.pool.clone())
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
                            army: army.clone(),
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
                            army,
                            bounty: None,
                            returns_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(CqrsError::domain_source)?;
            }
        }
        ScheduledActionPayload::AttackArrival {
            action_id,
            movement_id,
            army_id,
            return_action_id,
            village_id: _,
            source_village_id,
            target_village_id,
            player_id,
            army,
            attack_type,
            catapult_targets,
            arrives_at,
            returns_at,
        } => {
            svc.append_village_workflow_events(vec![(
                source_village_id,
                VillageEvent::AttackArrived {
                    movement_id,
                    army_id,
                    action_id,
                    return_action_id,
                    player_id,
                    source_village_id,
                    target_village_id,
                    army: army.clone(),
                    attack_type: attack_type.clone(),
                    catapult_targets: catapult_targets.clone(),
                    arrives_at,
                    returns_at,
                },
            )])
            .await?;
            let outcome = build_attack_outcome_command(
                svc,
                action_id,
                movement_id,
                return_action_id,
                army_id,
                player_id,
                source_village_id,
                target_village_id,
                army,
                attack_type,
                catapult_targets,
                returns_at,
            )
            .await?;
            svc.append_village_workflow_events(vec![
                (
                    source_village_id,
                    VillageEvent::AttackBattleResolved {
                        action_id: outcome.source.action_id,
                        movement_id: outcome.source.movement_id,
                        return_action_id: outcome.source.return_action_id,
                        army_id: outcome.source.army_id,
                        player_id: outcome.source.player_id,
                        source_village_id: outcome.source.source_village_id,
                        target_village_id: outcome.source.target_village_id,
                        attack_type: outcome.source.attack_type.clone(),
                        report: outcome.source.report.clone(),
                        returning_army: outcome.source.returning_army.clone(),
                        stationed_attacker_army: outcome.source.stationed_attacker_army.clone(),
                        returns_at: outcome.source.returns_at,
                    },
                ),
                (
                    target_village_id,
                    VillageEvent::BattleOutcomeAppliedToVillage {
                        action_id: outcome.target.action_id,
                        movement_id: outcome.target.movement_id,
                        source_village_id: outcome.target.source_village_id,
                        target_village_id: outcome.target.target_village_id,
                        target_player_id: outcome.target.target_player_id,
                        target_parent_village_id: outcome.target.target_parent_village_id,
                        target_loyalty: outcome.target.target_loyalty,
                        target_buildings: outcome.target.target_buildings.clone(),
                        target_production: outcome.target.target_production.clone(),
                        target_population: outcome.target.target_population,
                        target_stocks: outcome.target.target_stocks.clone(),
                        target_army: outcome.target.target_army.clone(),
                        target_reinforcements: outcome.target.target_reinforcements.clone(),
                        stationed_attacker_army: outcome.target.stationed_attacker_army.clone(),
                    },
                ),
            ])
            .await?;
        }
        ScheduledActionPayload::ArmyReturn {
            action_id,
            movement_id,
            army_id,
            village_id: _,
            source_village_id,
            target_village_id,
            player_id,
            army,
            returns_at,
            bounty,
        } => {
            let fact = build_army_return_fact(
                action_id,
                movement_id,
                army_id,
                player_id,
                source_village_id,
                target_village_id,
                army,
                bounty,
                returns_at,
            );
            svc.append_village_workflow_events(vec![(source_village_id, fact)])
                .await?;
        }
        ScheduledActionPayload::ScoutArrival {
            action_id,
            movement_id,
            army_id,
            return_action_id,
            village_id: _,
            source_village_id,
            target_village_id,
            player_id,
            army,
            target,
            attack_type,
            arrives_at,
            returns_at,
        } => {
            let outcome = build_scout_outcome_fact(
                svc,
                action_id,
                movement_id,
                return_action_id,
                army_id,
                player_id,
                source_village_id,
                target_village_id,
                army.clone(),
                target.clone(),
                attack_type.clone(),
                returns_at,
            )
            .await?;
            svc.append_village_workflow_events(vec![
                (
                    source_village_id,
                    VillageEvent::ScoutArrived {
                        movement_id,
                        army_id,
                        action_id,
                        return_action_id,
                        player_id,
                        source_village_id,
                        target_village_id,
                        army,
                        target,
                        attack_type,
                        arrives_at,
                        returns_at,
                    },
                ),
                (source_village_id, outcome.fact),
            ])
            .await?;
        }
        ScheduledActionPayload::MerchantsArrival {
            action_id,
            village_id: _,
            source_village_id,
            target_village_id,
            player_id,
            resources,
            merchants_used,
            arrives_at,
        } => {
            let (arrival_fact, applied_fact) = build_merchant_arrival_facts(
                svc,
                action_id,
                player_id,
                source_village_id,
                target_village_id,
                resources,
                merchants_used,
                arrives_at,
            )
            .await?;
            svc.append_village_workflow_events(vec![
                (source_village_id, arrival_fact),
                (target_village_id, applied_fact),
            ])
            .await?;
        }
        ScheduledActionPayload::MerchantsReturn {
            action_id,
            village_id: _,
            source_village_id,
            player_id,
            merchants_used,
            returns_at,
        } => {
            let fact = build_merchant_return_fact(
                action_id,
                player_id,
                source_village_id,
                merchants_used,
                returns_at,
            );
            svc.append_village_workflow_events(vec![(source_village_id, fact)])
                .await?;
        }
        ScheduledActionPayload::AddBuilding {
            village_id,
            player_id,
            slot_id,
            building_name,
            level,
            speed,
        } => {
            let fact = build_building_added_fact(
                action.id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
            );
            svc.append_village_workflow_events(vec![(village_id, fact)])
                .await?;
        }
        ScheduledActionPayload::UpgradeBuilding {
            village_id,
            player_id,
            slot_id,
            building_name,
            level,
            speed,
        } => {
            let fact = build_building_upgraded_fact(
                action.id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
            );
            svc.append_village_workflow_events(vec![(village_id, fact)])
                .await?;
        }
        ScheduledActionPayload::DowngradeBuilding {
            village_id,
            player_id,
            slot_id,
            building_name,
            level,
            speed,
        } => {
            let fact = build_building_downgraded_fact(
                action.id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
            );
            svc.append_village_workflow_events(vec![(village_id, fact)])
                .await?;
        }
        ScheduledActionPayload::TrainUnit {
            action_id,
            village_id,
            player_id,
            slot_id,
            unit,
            time_per_unit,
            quantity_remaining,
            execute_at,
        } => {
            let workflow_events = build_train_unit_completion_facts(
                action_id,
                player_id,
                village_id,
                slot_id,
                unit,
                time_per_unit,
                quantity_remaining,
                execute_at,
            );
            if !workflow_events.is_empty() {
                svc.append_village_workflow_events(workflow_events).await?;
            }
        }
        ScheduledActionPayload::ResearchAcademy {
            action_id,
            village_id,
            player_id,
            unit,
        } => {
            let fact =
                build_academy_research_completed_fact(action_id, player_id, village_id, unit);
            svc.append_village_workflow_events(vec![(village_id, fact)])
                .await?;
        }
        ScheduledActionPayload::ResearchSmithy {
            action_id,
            village_id,
            player_id,
            unit,
        } => {
            let fact = build_smithy_research_completed_fact(action_id, player_id, village_id, unit);
            svc.append_village_workflow_events(vec![(village_id, fact)])
                .await?;
        }
        ScheduledActionPayload::HeroRevival {
            action_id,
            village_id,
            player_id,
            hero,
            reset,
            revive_at,
        } => {
            let village = svc.get_village(village_id).await?;
            if village.player_id != player_id {
                return Err(CqrsError::domain_source(
                    parabellum_types::errors::GameError::VillageNotOwned {
                        village_id,
                        player_id,
                    },
                ));
            }
            if hero.player_id != player_id {
                return Err(CqrsError::domain_source(
                    parabellum_types::errors::GameError::HeroNotOwned {
                        hero_id: hero.id,
                        player_id,
                    },
                ));
            }

            let mut revived_hero = hero;
            revived_hero.resurrect(village_id, reset);
            svc.append_village_workflow_events(vec![(
                village_id,
                VillageEvent::HeroRevived {
                    action_id,
                    player_id,
                    village_id,
                    hero: revived_hero,
                    reset,
                    revived_at: revive_at,
                },
            )])
            .await?;
        }
    }
    Ok(())
}

async fn build_attack_outcome_command(
    svc: &VillageEsService,
    action_id: uuid::Uuid,
    movement_id: uuid::Uuid,
    return_action_id: uuid::Uuid,
    army_id: uuid::Uuid,
    player_id: uuid::Uuid,
    source_village_id: u32,
    target_village_id: u32,
    army: Army,
    attack_type: parabellum_types::battle::AttackType,
    catapult_targets: [parabellum_types::buildings::BuildingName; 2],
    returns_at: chrono::DateTime<chrono::Utc>,
) -> Result<ComputedAttackOutcome, CqrsError> {
    let source = svc.get_village(source_village_id).await?;
    let mut target = svc.get_village(target_village_id).await?;
    let army_repo = PostgresArmyRepository::new(svc.pool.clone());
    let target_home_army = army_repo
        .get_home_army(target_village_id)
        .await
        .map_err(CqrsError::domain_source)?;
    let target_reinforcements = army_repo
        .list_stationed_armies(target_village_id)
        .await
        .map_err(CqrsError::domain_source)?;
    target.army = target_home_army;
    target.reinforcements = target_reinforcements;
    let can_attempt_conquer = attack_type == parabellum_types::battle::AttackType::Normal
        && can_attempt_conquer(svc, &source, &target, &army).await?;

    let attacker_village =
        Village::try_from(source.clone()).map_err(CqrsError::domain_source)?;
    let mut defender_village =
        Village::try_from(target.clone()).map_err(CqrsError::domain_source)?;
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
        attack_type.clone(),
        attacker_army.clone(),
        attacker_village,
        defender_village.clone(),
        selected_targets,
        can_attempt_conquer,
    );
    let report = battle.calculate_battle();
    attacker_army.apply_battle_report(&report.attacker);
    let _ = defender_village.apply_battle_report(&report, 1);

    let conquered = can_attempt_conquer && report.loyalty_after == 0;
    let mut target_player_id = target.player_id;
    let mut target_parent_village_id = target.parent_village_id;
    let mut target_loyalty = defender_village.loyalty();
    let mut target_army = defender_village.army().cloned();
    let mut target_reinforcements = defender_village.reinforcements().clone();
    if conquered {
        target_player_id = player_id;
        target_parent_village_id = Some(source_village_id);
        target_loyalty = 100;
        target_army = None;
        target_reinforcements = vec![];
    }

    let stationed_attacker_army = if conquered {
        let mut stationed = attacker_army.clone();
        let mut units = stationed.units().clone();
        units.set(8, 0);
        stationed.update_units(&units);
        if stationed.immensity() > 0 {
            Some(stationed)
        } else {
            None
        }
    } else {
        None
    };

    let returning_army = if conquered || attacker_army.immensity() == 0 {
        None
    } else {
        Some(Army::new(
            Some(movement_id),
            source_village_id,
            Some(target_village_id),
            player_id,
            source.tribe.clone(),
            attacker_army.units(),
            &no_smithy,
            attacker_army.hero(),
        ))
    };

    let stationed_attacker_for_target = stationed_attacker_army.clone();
    Ok(ComputedAttackOutcome {
        source: ResolveAttackBattle {
            action_id,
            movement_id,
            return_action_id,
            army_id,
            player_id,
            source_village_id,
            target_village_id,
            attack_type,
            report,
            returning_army,
            stationed_attacker_army,
            returns_at,
        },
        target: ApplyBattleOutcomeToVillage {
            action_id,
            movement_id,
            source_village_id,
            target_village_id,
            target_player_id,
            target_parent_village_id,
            target_loyalty,
            target_buildings: defender_village.buildings().to_vec(),
            target_production: defender_village.production.clone(),
            target_population: defender_village.population,
            target_stocks: defender_village.stocks().clone(),
            target_army,
            target_reinforcements,
            stationed_attacker_army: stationed_attacker_for_target,
        },
    })
}

#[allow(clippy::too_many_arguments)]
async fn build_merchant_arrival_facts(
    svc: &VillageEsService,
    action_id: uuid::Uuid,
    player_id: uuid::Uuid,
    source_village_id: u32,
    target_village_id: u32,
    resources: parabellum_types::common::ResourceGroup,
    merchants_used: u8,
    arrives_at: chrono::DateTime<chrono::Utc>,
) -> Result<(VillageEvent, VillageEvent), CqrsError> {
    let target = svc.get_village(target_village_id).await?;
    let target_stocks = parabellum_game::models::village::VillageStocks {
        warehouse_capacity: target.stocks.warehouse_capacity,
        granary_capacity: target.stocks.granary_capacity,
        lumber: target.stocks.lumber.saturating_add(resources.lumber()),
        clay: target.stocks.clay.saturating_add(resources.clay()),
        iron: target.stocks.iron.saturating_add(resources.iron()),
        crop: target.stocks.crop.saturating_add(resources.crop() as i64),
    };

    Ok((
        VillageEvent::MerchantsArrived {
            action_id,
            player_id,
            source_village_id,
            target_village_id,
            resources: resources.clone(),
            merchants_used,
            arrives_at,
        },
        VillageEvent::MerchantTransferAppliedToVillage {
            action_id,
            player_id,
            source_village_id,
            target_village_id,
            resources,
            merchants_used,
            arrives_at,
            target_stocks,
        },
    ))
}

#[allow(clippy::too_many_arguments)]
async fn build_scout_outcome_fact(
    svc: &VillageEsService,
    action_id: uuid::Uuid,
    movement_id: uuid::Uuid,
    return_action_id: uuid::Uuid,
    army_id: uuid::Uuid,
    player_id: uuid::Uuid,
    source_village_id: u32,
    target_village_id: u32,
    army: Army,
    target: parabellum_types::battle::ScoutingTarget,
    attack_type: parabellum_types::battle::AttackType,
    returns_at: chrono::DateTime<chrono::Utc>,
) -> Result<ComputedScoutOutcome, CqrsError> {
    let source = svc.get_village(source_village_id).await?;
    let target_village_model = svc.get_village(target_village_id).await?;
    let attacker_village =
        Village::try_from(source.clone()).map_err(CqrsError::domain_source)?;
    let defender_village =
        Village::try_from(target_village_model).map_err(CqrsError::domain_source)?;
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
        false,
    );
    let report = battle.calculate_scout_battle(target);
    attacker_army.apply_battle_report(&report.attacker);

    let returning_army = if attacker_army.immensity() == 0 {
        None
    } else {
        Some(Army::new(
            Some(movement_id),
            source_village_id,
            Some(target_village_id),
            player_id,
            source.tribe.clone(),
            attacker_army.units(),
            &no_smithy,
            attacker_army.hero(),
        ))
    };

    Ok(ComputedScoutOutcome {
        fact: VillageEvent::ScoutBattleResolved {
            action_id,
            movement_id,
            return_action_id,
            army_id,
            player_id,
            source_village_id,
            target_village_id,
            attack_type,
            report,
            returning_army,
            returns_at,
        },
    })
}

async fn can_attempt_conquer(
    svc: &VillageEsService,
    source: &VillageModel,
    target: &VillageModel,
    attacking_army: &Army,
) -> Result<bool, CqrsError> {
    if target.is_capital {
        return Ok(false);
    }
    if attacking_army.get_troop_count_by_role(UnitRole::Chief) == 0 {
        return Ok(false);
    }

    let source_village = Village::try_from(source.clone()).map_err(CqrsError::domain_source)?;
    let max_slots = source_village.max_foundation_slots() as usize;
    if max_slots == 0 {
        return Ok(false);
    }

    let village_repo = PostgresVillageRepository::new(svc.pool.clone());
    let player_villages = village_repo
        .list_by_player_id(source.player_id)
        .await
        .map_err(CqrsError::domain_source)?;
    let used_slots = player_villages
        .iter()
        .filter(|v| v.parent_village_id == Some(source.village_id))
        .count();
    if used_slots >= max_slots {
        return Ok(false);
    }

    let player_repo = PostgresPlayerRepository::new(svc.pool.clone());
    player_repo
        .update_culture_points(source.player_id)
        .await
        .map_err(CqrsError::domain_source)?;
    let total_cp = player_repo
        .get_by_id(source.player_id)
        .await
        .map_err(CqrsError::domain_source)?
        .culture_points;
    let needed_cp = required_cp(
        parabellum_types::common::Speed::X1,
        player_villages.len() + 1,
    );
    Ok(total_cp >= needed_cp)
}
