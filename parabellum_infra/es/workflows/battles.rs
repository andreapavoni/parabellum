//! Battle and scout scheduled workflow orchestration.
//!
//! The actual combat math remains in `parabellum_game::battle`; this module
//! loads the read-side state required to construct domain villages, asks the
//! domain to resolve the encounter, then converts app command outcomes into
//! canonical aggregate events.

use std::collections::HashMap;

use mini_cqrs_es::CqrsError;
use parabellum_app::ports::identity::PlayerRepository;
use parabellum_app::villages::models::{
    AttackArrivalWorkflow, ScoutArrivalWorkflow, TrappedTroopReturn, VillageModel,
};
use parabellum_app::villages::repositories::{ArmyRepository, VillageRepository};
use parabellum_app::villages::{
    ApplyBattleOutcomeToVillage, ConquestAttempt, ResolveAttackBattle, ResolveScoutBattle,
    VillageArmyContext, hydrate_village,
};
use parabellum_game::battle::Battle;
use parabellum_game::models::army::Army;
use parabellum_game::models::buildings::Building;
use parabellum_game::models::smithy::SmithyUpgrades;
use parabellum_game::models::trapper::Trapper;
use parabellum_game::models::village::Village;
use parabellum_types::army::{TroopSet, UnitRole};
use parabellum_types::battle::AttackType;

use crate::es::{PostgresArmyRepository, PostgresVillageRepository, VillageEsService};
use crate::identity::repositories::PostgresPlayerRepository;

#[derive(Debug)]
pub(crate) struct AttackOutcome {
    source: ResolveAttackBattle,
    target: ApplyBattleOutcomeToVillage,
}

impl AttackOutcome {
    fn into_events(self) -> super::WorkflowEvents {
        let source_village_id = self.source.source_village_id;
        let target_village_id = self.target.target_village_id;

        super::WorkflowEvents::from_events(vec![
            (source_village_id, self.source.into_outcome_event()),
            (target_village_id, self.target.into_outcome_event()),
        ])
    }
}

pub(crate) async fn resolve_attack(
    svc: &VillageEsService,
    workflow: AttackArrivalWorkflow,
) -> Result<super::WorkflowEvents, CqrsError> {
    let outcome = build_attack_outcome(svc, workflow).await?;
    Ok(outcome.into_events())
}

pub(crate) async fn resolve_scout(
    svc: &VillageEsService,
    workflow: ScoutArrivalWorkflow,
) -> Result<super::WorkflowEvents, CqrsError> {
    let outcome = build_scout_outcome(svc, workflow).await?;
    Ok(scout_resolution_events(outcome))
}

async fn hydrate_village_with_current_armies(
    svc: &VillageEsService,
    model: VillageModel,
) -> Result<Village, CqrsError> {
    let army_repo = PostgresArmyRepository::new(svc.pool().clone());
    let village_id = model.village_id;
    let armies = army_repo
        .army_context_for_village(village_id)
        .await
        .map_err(CqrsError::domain_source)?;
    Ok(hydrate_village(model, armies))
}

pub(crate) fn scout_resolution_events(source: ResolveScoutBattle) -> super::WorkflowEvents {
    let source_village_id = source.source_village_id;

    super::WorkflowEvents::one(source_village_id, source.into_outcome_event())
}

async fn build_attack_outcome(
    svc: &VillageEsService,
    workflow: AttackArrivalWorkflow,
) -> Result<AttackOutcome, CqrsError> {
    let source = svc.get_village(workflow.source_village_id).await?;
    let target = svc.get_village(workflow.target_village_id).await?;
    let can_attempt_conquer = workflow.attack_type == parabellum_types::battle::AttackType::Normal
        && can_attempt_conquer(svc, &source, &target, &workflow.army).await?;

    let attacker_village = hydrate_village(source.clone(), VillageArmyContext::default());
    let defender_armies = PostgresArmyRepository::new(svc.pool().clone())
        .army_context_for_village(target.village_id)
        .await
        .map_err(CqrsError::domain_source)?;
    let trapped_here = defender_armies.trapped_here.clone();
    let occupied_traps = trapped_here
        .iter()
        .map(|army| army.units().immensity())
        .sum();
    let mut trapper = Trapper::from_buildings(&target.buildings, target.trapper, occupied_traps);
    let mut defender_village = hydrate_village(target.clone(), defender_armies);
    let no_smithy: SmithyUpgrades = [0; 8];
    let mut attacker_army = Army::new(
        Some(workflow.army_id),
        workflow.army.village_id,
        workflow.army.current_map_field_id,
        workflow.army.player_id,
        workflow.army.tribe.clone(),
        workflow.army.units(),
        workflow.army.smithy(),
        workflow.army.hero(),
    );

    let mut captured_army = None;
    let capture = trapper.capture(attacker_army.units());
    if capture.traps_used > 0 {
        let captured = attacker_army
            .split_units(
                capture.trapped_units.clone(),
                None,
                workflow.target_village_id,
            )
            .map_err(CqrsError::domain_source)?;
        captured_army = Some(Army::new(
            Some(captured.id),
            captured.village_id,
            Some(workflow.target_village_id),
            captured.player_id,
            captured.tribe.clone(),
            captured.units(),
            captured.smithy(),
            captured.hero(),
        ));
    }

    let mut selected_targets: Vec<Building> = Vec::new();
    for name in workflow.catapult_targets {
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
    let mut report = if attacker_army.immensity() == 0 {
        parabellum_game::battle::BattleReport {
            attack_type: workflow.attack_type.clone(),
            attacker: parabellum_game::battle::BattlePartyReport {
                army_before: attacker_army.clone(),
                survivors: TroopSet::default(),
                losses: TroopSet::default(),
                hero_exp_gained: 0,
                loss_percentage: 0.0,
            },
            defender: defender_village.army().cloned().map(|army| {
                parabellum_game::battle::BattlePartyReport {
                    survivors: army.units().clone(),
                    losses: TroopSet::default(),
                    army_before: army,
                    hero_exp_gained: 0,
                    loss_percentage: 0.0,
                }
            }),
            reinforcements: defender_village
                .reinforcements()
                .iter()
                .cloned()
                .map(|army| parabellum_game::battle::BattlePartyReport {
                    survivors: army.units().clone(),
                    losses: TroopSet::default(),
                    army_before: army,
                    hero_exp_gained: 0,
                    loss_percentage: 0.0,
                })
                .collect(),
            scouting: None,
            bounty: Some(Default::default()),
            wall_damage: None,
            catapult_damage: vec![],
            loyalty_before: defender_village.loyalty(),
            loyalty_after: defender_village.loyalty(),
            trapped: None,
            freed: None,
        }
    } else {
        let battle = Battle::new(
            workflow.attack_type.clone(),
            attacker_army.clone(),
            attacker_village,
            defender_village.clone(),
            selected_targets,
            can_attempt_conquer,
        );
        let report = battle.calculate_battle();
        attacker_army.apply_battle_report(&report.attacker);
        let _ = defender_village.apply_battle_report(&report, 1);
        report
    };
    if capture.traps_used > 0 {
        report.trapped = Some(capture);
    }

    let mut freed_trapped: Vec<Army> =
        if workflow.attack_type == AttackType::Normal && attacker_army.immensity() > 0 {
            trapped_here
                .iter()
                .filter(|army| army.player_id == workflow.player_id)
                .cloned()
                .collect()
        } else {
            vec![]
        };
    let captured_freed_by_attack = workflow.attack_type == AttackType::Normal
        && attacker_army.immensity() > 0
        && captured_army.is_some();
    if captured_freed_by_attack && let Some(captured) = captured_army.clone() {
        freed_trapped.push(captured);
    }
    let mut freed_trapped_army_ids = Vec::new();
    let mut freed_trapped_returns = Vec::new();
    if !freed_trapped.is_empty() {
        let mut freed_units = TroopSet::default();
        for army in &freed_trapped {
            for (idx, quantity) in army.units().units().iter().enumerate() {
                freed_units.add(idx, *quantity);
            }
            freed_trapped_army_ids.push(army.id);
        }
        let free = trapper.free_by_attack(&freed_units);
        let survivor_units_by_home = freed_survivor_units_by_home(&freed_trapped, &free.survivors);
        for (home_village_id, survivors) in survivor_units_by_home {
            if survivors.immensity() == 0 {
                continue;
            }
            let freed_survivor_army = Army::new(
                None,
                home_village_id,
                Some(workflow.target_village_id),
                workflow.player_id,
                source.tribe.clone(),
                &survivors,
                &no_smithy,
                None,
            );
            if home_village_id == workflow.source_village_id {
                attacker_army
                    .merge(&freed_survivor_army)
                    .map_err(CqrsError::domain_source)?;
            } else {
                freed_trapped_returns.push(TrappedTroopReturn {
                    action_id: uuid::Uuid::new_v4(),
                    movement_id: uuid::Uuid::new_v4(),
                    army_id: freed_survivor_army.id,
                    player_id: workflow.player_id,
                    home_village_id,
                    trapped_village_id: workflow.target_village_id,
                    army: freed_survivor_army,
                    returns_at: workflow.returns_at,
                });
            }
        }
        report.freed = Some(free);
    }
    if captured_freed_by_attack {
        captured_army = None;
    }

    let conquered = can_attempt_conquer && report.loyalty_after == 0;
    let mut target_player_id = target.player_id;
    let mut target_tribe = target.tribe.clone();
    let mut target_parent_village_id = target.parent_village_id;
    let mut target_loyalty = defender_village.loyalty();
    let mut target_army = defender_village.army().cloned();
    let mut target_reinforcements = defender_village.reinforcements().clone();
    if conquered {
        target_player_id = workflow.player_id;
        target_tribe = source.tribe.clone();
        target_parent_village_id = Some(workflow.source_village_id);
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
            Some(workflow.movement_id),
            workflow.source_village_id,
            Some(workflow.target_village_id),
            workflow.player_id,
            source.tribe.clone(),
            attacker_army.units(),
            &no_smithy,
            attacker_army.hero(),
        ))
    };

    let stationed_attacker_for_target = stationed_attacker_army.clone();
    Ok(AttackOutcome {
        source: ResolveAttackBattle {
            action_id: workflow.action_id,
            movement_id: workflow.movement_id,
            return_action_id: workflow.return_action_id,
            army_id: workflow.army_id,
            player_id: workflow.player_id,
            source_village_id: workflow.source_village_id,
            target_village_id: workflow.target_village_id,
            attack_type: workflow.attack_type,
            report,
            returning_army,
            trapped_attacker_army: captured_army,
            freed_trapped_army_ids,
            freed_trapped_returns,
            stationed_attacker_army,
            returns_at: workflow.returns_at,
        },
        target: ApplyBattleOutcomeToVillage {
            action_id: workflow.action_id,
            movement_id: workflow.movement_id,
            source_village_id: workflow.source_village_id,
            target_village_id: workflow.target_village_id,
            target_player_id,
            target_tribe,
            target_parent_village_id,
            target_loyalty,
            target_buildings: defender_village.buildings().to_vec(),
            target_production: defender_village.production.clone(),
            target_population: defender_village.population,
            target_stocks: defender_village.stocks().clone(),
            target_trapper: trapper.state(),
            target_army,
            target_reinforcements,
            stationed_attacker_army: stationed_attacker_for_target,
        },
    })
}

async fn build_scout_outcome(
    svc: &VillageEsService,
    workflow: ScoutArrivalWorkflow,
) -> Result<ResolveScoutBattle, CqrsError> {
    let source = svc.get_village(workflow.source_village_id).await?;
    let target_village_model = svc.get_village(workflow.target_village_id).await?;
    let attacker_village = hydrate_village(source.clone(), VillageArmyContext::default());
    let defender_village = hydrate_village_with_current_armies(svc, target_village_model).await?;
    let no_smithy: SmithyUpgrades = [0; 8];
    let mut attacker_army = Army::new(
        Some(workflow.army_id),
        workflow.army.village_id,
        workflow.army.current_map_field_id,
        workflow.army.player_id,
        workflow.army.tribe.clone(),
        workflow.army.units(),
        workflow.army.smithy(),
        workflow.army.hero(),
    );
    let battle = Battle::new(
        workflow.attack_type.clone(),
        attacker_army.clone(),
        attacker_village,
        defender_village,
        None,
        false,
    );
    let report = battle.calculate_scout_battle(workflow.target);
    attacker_army.apply_battle_report(&report.attacker);

    let returning_army = if attacker_army.immensity() == 0 {
        None
    } else {
        Some(Army::new(
            Some(workflow.movement_id),
            workflow.source_village_id,
            Some(workflow.target_village_id),
            workflow.player_id,
            source.tribe.clone(),
            attacker_army.units(),
            &no_smithy,
            attacker_army.hero(),
        ))
    };

    Ok(ResolveScoutBattle {
        action_id: workflow.action_id,
        movement_id: workflow.movement_id,
        return_action_id: workflow.return_action_id,
        army_id: workflow.army_id,
        player_id: workflow.player_id,
        source_village_id: workflow.source_village_id,
        target_village_id: workflow.target_village_id,
        attack_type: workflow.attack_type,
        report,
        returning_army,
        returns_at: workflow.returns_at,
    })
}

fn freed_survivor_units_by_home(
    trapped_armies: &[Army],
    survivors: &TroopSet,
) -> HashMap<u32, TroopSet> {
    let mut remaining = survivors.clone();
    let mut by_home: HashMap<u32, TroopSet> = HashMap::new();

    for army in trapped_armies {
        let entry = by_home.entry(army.village_id).or_default();
        for (idx, trapped_quantity) in army.units().units().iter().enumerate() {
            let assignable = remaining.get(idx).min(*trapped_quantity);
            if assignable > 0 {
                entry.add(idx, assignable);
                remaining.remove(idx, assignable);
            }
        }
    }

    by_home
}

async fn can_attempt_conquer(
    svc: &VillageEsService,
    source: &VillageModel,
    target: &VillageModel,
    attacking_army: &Army,
) -> Result<bool, CqrsError> {
    Ok(load_conquest_attempt(svc, source, target, attacking_army)
        .await?
        .is_allowed())
}

async fn load_conquest_attempt(
    svc: &VillageEsService,
    source: &VillageModel,
    target: &VillageModel,
    attacking_army: &Army,
) -> Result<ConquestAttempt, CqrsError> {
    let source_village = hydrate_village(source.clone(), VillageArmyContext::default());
    let village_repo = PostgresVillageRepository::new(svc.pool().clone());
    let ownership = village_repo
        .get_expansion_ownership_snapshot(source.player_id, source.village_id)
        .await
        .map_err(CqrsError::domain_source)?;

    let cfg = parabellum_app::config::Config::from_env();
    Ok(ConquestAttempt {
        target_is_capital: target.is_capital,
        attacking_chiefs: attacking_army.get_troop_count_by_role(UnitRole::Chief),
        source_max_slots: source_village.max_foundation_slots(),
        source_child_villages: ownership.source_child_villages,
        player_village_count: ownership.player_village_count,
        player_culture_points: refresh_player_culture_points(svc, source.player_id).await?,
        speed: parabellum_types::common::Speed::from(cfg.speed),
    })
}

async fn refresh_player_culture_points(
    svc: &VillageEsService,
    player_id: uuid::Uuid,
) -> Result<u32, CqrsError> {
    let player_repo = PostgresPlayerRepository::new(svc.pool().clone());
    player_repo
        .update_culture_points(player_id)
        .await
        .map_err(CqrsError::domain_source)?;
    Ok(player_repo
        .get_by_id(player_id)
        .await
        .map_err(CqrsError::domain_source)?
        .culture_points)
}
