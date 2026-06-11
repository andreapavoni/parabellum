//! Battle and scout scheduled workflow orchestration.
//!
//! The actual combat math remains in `parabellum_game::battle`; this module
//! loads the read-side state required to construct domain villages, asks the
//! domain to resolve the encounter, then converts app command outcomes into
//! canonical aggregate events.

use mini_cqrs_es::CqrsError;
use parabellum_app::ports::identity::PlayerRepository;
use parabellum_app::villages::models::{AttackArrivalWorkflow, ScoutArrivalWorkflow, VillageModel};
use parabellum_app::villages::repositories::{ArmyRepository, VillageRepository};
use parabellum_app::villages::{
    ApplyBattleOutcomeToVillage, ConquestAttempt, ResolveAttackBattle, ResolveScoutBattle,
    hydrate_village,
};
use parabellum_game::battle::Battle;
use parabellum_game::models::army::Army;
use parabellum_game::models::buildings::Building;
use parabellum_game::models::smithy::SmithyUpgrades;
use parabellum_game::models::village::Village;
use parabellum_types::army::UnitRole;

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

    let attacker_village = Village::try_from(source.clone()).map_err(CqrsError::domain_source)?;
    let mut defender_village = hydrate_village_with_current_armies(svc, target.clone()).await?;
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
    let attacker_village = Village::try_from(source.clone()).map_err(CqrsError::domain_source)?;
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
    let source_village = Village::try_from(source.clone()).map_err(CqrsError::domain_source)?;
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
