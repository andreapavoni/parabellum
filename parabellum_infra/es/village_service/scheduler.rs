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
use crate::es::workflows;
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
        ScheduledActionPayload::ReinforcementArrival { workflow } => {
            let source_village_id = workflow.source_village_id;
            let target_village_id = workflow.target_village_id;
            let source = svc.get_village(source_village_id).await?;
            let target = svc.get_village(target_village_id).await?;
            svc.append_village_workflow_events(workflows::movements::reinforcement_arrival_facts(
                workflow, &source, &target,
            ))
            .await?;
        }
        ScheduledActionPayload::SettlersArrival { workflow } => {
            let source_village_id = workflow.source_village_id;
            let target_village_id = workflow.target_village_id;
            let player_id = workflow.player_id;
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

                svc.append_village_workflow_events(
                    workflows::movements::settlers_foundation_facts(workflow, default_buildings),
                )
                .await?;
            } else {
                tracing::warn!(
                    action_id = %workflow.action_id,
                    player_id = %player_id,
                    source_village_id,
                    target_village_id,
                    "settlers arrival target unavailable, scheduling army return"
                );
                let army_repo: Arc<dyn ArmyRepository> =
                    Arc::new(PostgresArmyRepository::new(svc.pool.clone()));
                let army = army_repo
                    .get_moving_army(workflow.army_id)
                    .await
                    .map_err(CqrsError::domain_source)?;
                let source = svc.get_village(source_village_id).await?;
                let cfg = parabellum_app::config::Config::from_env();
                let travel_secs = source.position.calculate_travel_time_secs(
                    workflow.target_position.clone(),
                    army.speed(),
                    cfg.world_size as i32,
                    cfg.speed as u8,
                ) as i64;
                let returns_at =
                    workflow.arrives_at + chrono::Duration::seconds(std::cmp::max(1, travel_secs));
                let return_action_id = uuid::Uuid::new_v4();
                let return_action = workflows::movements::army_return_scheduled_action(
                    return_action_id,
                    workflow.movement_id,
                    workflow.army_id,
                    source_village_id,
                    source_village_id,
                    target_village_id,
                    player_id,
                    army,
                    None,
                    returns_at,
                )?;
                PostgresScheduledActionRepository::new(svc.pool.clone())
                    .add(&return_action)
                    .await
                    .map_err(CqrsError::domain_source)?;
            }
        }
        ScheduledActionPayload::AttackArrival { workflow } => {
            let source_village_id = workflow.source_village_id;
            let target_village_id = workflow.target_village_id;
            svc.append_village_workflow_events(vec![(
                source_village_id,
                workflows::movements::attack_arrived_fact(&workflow),
            )])
            .await?;
            let outcome = build_attack_outcome_command(
                svc,
                workflow.action_id,
                workflow.movement_id,
                workflow.return_action_id,
                workflow.army_id,
                workflow.player_id,
                workflow.source_village_id,
                workflow.target_village_id,
                workflow.army,
                workflow.attack_type,
                workflow.catapult_targets,
                workflow.returns_at,
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
                        target_tribe: outcome.target.target_tribe.clone(),
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
        ScheduledActionPayload::ArmyReturn { workflow } => {
            let source_village_id = workflow.source_village_id;
            let fact = workflows::movements::army_return_fact(action.id, workflow);
            svc.append_village_workflow_events(vec![(source_village_id, fact)])
                .await?;
        }
        ScheduledActionPayload::ScoutArrival { workflow } => {
            let source_village_id = workflow.source_village_id;
            let outcome = build_scout_outcome_fact(
                svc,
                workflow.action_id,
                workflow.movement_id,
                workflow.return_action_id,
                workflow.army_id,
                workflow.player_id,
                workflow.source_village_id,
                workflow.target_village_id,
                workflow.army.clone(),
                workflow.target.clone(),
                workflow.attack_type.clone(),
                workflow.returns_at,
            )
            .await?;
            svc.append_village_workflow_events(vec![
                (
                    source_village_id,
                    workflows::movements::scout_arrived_fact(&workflow),
                ),
                (source_village_id, outcome.fact),
            ])
            .await?;
        }
        ScheduledActionPayload::MerchantsArrival { workflow } => {
            let source_village_id = workflow.source_village_id;
            let target_village_id = workflow.target_village_id;
            let target = svc.get_village(target_village_id).await?;
            let (arrival_fact, applied_fact) =
                workflows::merchants::arrival_facts(action.id, workflow, &target);
            svc.append_village_workflow_events(vec![
                (source_village_id, arrival_fact),
                (target_village_id, applied_fact),
            ])
            .await?;
        }
        ScheduledActionPayload::MerchantsReturn { workflow } => {
            let source_village_id = workflow.source_village_id;
            let fact = workflows::merchants::return_fact(action.id, workflow);
            svc.append_village_workflow_events(vec![(source_village_id, fact)])
                .await?;
        }
        ScheduledActionPayload::Building { workflow } => {
            let village_id = workflow.village_id;
            let fact = workflows::buildings::completion_fact(action.id, workflow);
            svc.append_village_workflow_events(vec![(village_id, fact)])
                .await?;
        }
        ScheduledActionPayload::Training { workflow } => {
            let workflow_events = workflows::training::completion_facts(action.id, workflow);
            if !workflow_events.is_empty() {
                svc.append_village_workflow_events(workflow_events).await?;
            }
        }
        ScheduledActionPayload::Research { workflow } => {
            let village_id = workflow.village_id;
            let fact = workflows::research::completion_fact(action.id, workflow);
            svc.append_village_workflow_events(vec![(village_id, fact)])
                .await?;
        }
        ScheduledActionPayload::HeroRevival { workflow } => {
            let village_id = workflow.village_id;
            let player_id = workflow.player_id;
            let hero_id = workflow.hero.id;
            let village = svc.get_village(village_id).await?;
            if village.player_id != player_id {
                return Err(CqrsError::domain_source(
                    parabellum_types::errors::GameError::VillageNotOwned {
                        village_id,
                        player_id,
                    },
                ));
            }
            if workflow.hero.player_id != player_id {
                return Err(CqrsError::domain_source(
                    parabellum_types::errors::GameError::HeroNotOwned { hero_id, player_id },
                ));
            }

            svc.append_village_workflow_events(vec![workflows::heroes::revived_fact(
                action.id, workflow,
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
    catapult_targets: [Option<parabellum_types::buildings::BuildingName>; 2],
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

    let attacker_village = Village::try_from(source.clone()).map_err(CqrsError::domain_source)?;
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
        match name {
            Some(name) => match defender_village.get_building_by_name(&name) {
                Some(slot) => selected_targets.push(slot.building.clone()),
                None => {
                    if let Some(random) = defender_village.get_random_buildings(1).pop() {
                        selected_targets.push(random);
                    }
                }
            },
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
    let mut target_tribe = target.tribe.clone();
    let mut target_parent_village_id = target.parent_village_id;
    let mut target_loyalty = defender_village.loyalty();
    let mut target_army = defender_village.army().cloned();
    let mut target_reinforcements = defender_village.reinforcements().clone();
    if conquered {
        target_player_id = player_id;
        target_tribe = source.tribe.clone();
        target_parent_village_id = Some(source_village_id);
        target_loyalty = 0;
        target_army = None;
        target_reinforcements = vec![];
    }

    let stationed_attacker_army = if conquered {
        let mut stationed = attacker_army.clone();
        let mut units = stationed.units().clone();
        let chiefs = units.get(8);
        if chiefs > 0 {
            units.set(8, chiefs - 1);
        }
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
            target_tribe,
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
    let attacker_village = Village::try_from(source.clone()).map_err(CqrsError::domain_source)?;
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
    let cfg = parabellum_app::config::Config::from_env();
    let needed_cp = required_cp(
        parabellum_types::common::Speed::from(cfg.speed),
        player_villages.len() + 1,
    );
    Ok(total_cp >= needed_cp)
}
