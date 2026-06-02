use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    ArmyReturnWorkflow, AttackArrivalWorkflow, ReinforcementArrivalWorkflow, ScheduledAction,
    ScheduledActionPayload, ScoutArrivalWorkflow, SettlersArrivalWorkflow, VillageModel,
};
use parabellum_game::models::army::Army;
use parabellum_types::common::ResourceGroup;
use uuid::Uuid;

#[allow(clippy::too_many_arguments)]
pub(crate) fn army_return_scheduled_action(
    action_id: Uuid,
    movement_id: Uuid,
    army_id: Uuid,
    village_id: u32,
    source_village_id: u32,
    target_village_id: u32,
    player_id: Uuid,
    army: Army,
    bounty: Option<ResourceGroup>,
    returns_at: chrono::DateTime<chrono::Utc>,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        action_id,
        returns_at,
        ScheduledActionPayload::ArmyReturn {
            workflow: ArmyReturnWorkflow {
                village_id,
                movement_id,
                army_id,
                source_village_id,
                target_village_id,
                player_id,
                army,
                bounty,
                returns_at,
            },
        },
    )
}

pub(crate) fn army_return_fact(action_id: Uuid, workflow: ArmyReturnWorkflow) -> VillageEvent {
    VillageEvent::ArmyReturned {
        action_id,
        movement_id: workflow.movement_id,
        army_id: workflow.army_id,
        player_id: workflow.player_id,
        source_village_id: workflow.source_village_id,
        target_village_id: workflow.target_village_id,
        army: workflow.army,
        bounty: workflow.bounty,
        returns_at: workflow.returns_at,
    }
}

pub(crate) fn reinforcement_arrival_scheduled_action(
    workflow: ReinforcementArrivalWorkflow,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        workflow.movement_id,
        workflow.arrives_at,
        ScheduledActionPayload::ReinforcementArrival { workflow },
    )
}

pub(crate) fn reinforcement_arrival_workflow(
    movement_id: Uuid,
    army_id: Uuid,
    player_id: Uuid,
    source_village_id: u32,
    target_village_id: u32,
    army: Army,
    arrives_at: chrono::DateTime<chrono::Utc>,
) -> ReinforcementArrivalWorkflow {
    ReinforcementArrivalWorkflow {
        movement_id,
        army_id,
        player_id,
        source_village_id,
        target_village_id,
        army,
        arrives_at,
    }
}

pub(crate) fn reinforcement_arrival_facts(
    workflow: ReinforcementArrivalWorkflow,
    source: &VillageModel,
    target: &VillageModel,
) -> Vec<(u32, VillageEvent)> {
    let hero_alone_transfer = workflow.army.hero().is_some()
        && workflow.army.units().immensity() == 0
        && source.player_id == target.player_id
        && source.buildings.iter().any(|b| {
            b.building.name == parabellum_types::buildings::BuildingName::HeroMansion
                && b.building.level > 0
        })
        && target.buildings.iter().any(|b| {
            b.building.name == parabellum_types::buildings::BuildingName::HeroMansion
                && b.building.level > 0
        });

    vec![
        (
            workflow.source_village_id,
            VillageEvent::ReinforcementArrived {
                movement_id: workflow.movement_id,
                army_id: workflow.army_id,
                player_id: workflow.player_id,
                source_village_id: workflow.source_village_id,
                target_village_id: workflow.target_village_id,
                army: workflow.army.clone(),
                hero_alone_transfer,
                arrives_at: workflow.arrives_at,
            },
        ),
        (
            workflow.target_village_id,
            VillageEvent::ReinforcementAppliedToVillage {
                movement_id: workflow.movement_id,
                army_id: workflow.army_id,
                player_id: workflow.player_id,
                source_village_id: workflow.source_village_id,
                target_village_id: workflow.target_village_id,
                army: workflow.army,
                hero_alone_transfer,
                arrives_at: workflow.arrives_at,
            },
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn scout_arrival_scheduled_action(
    action_id: Uuid,
    movement_id: Uuid,
    army_id: Uuid,
    return_action_id: Uuid,
    village_id: u32,
    source_village_id: u32,
    target_village_id: u32,
    player_id: Uuid,
    army: Army,
    target: parabellum_types::battle::ScoutingTarget,
    attack_type: parabellum_types::battle::AttackType,
    arrives_at: chrono::DateTime<chrono::Utc>,
    returns_at: chrono::DateTime<chrono::Utc>,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        action_id,
        arrives_at,
        ScheduledActionPayload::ScoutArrival {
            workflow: ScoutArrivalWorkflow {
                action_id,
                movement_id,
                army_id,
                return_action_id,
                village_id,
                source_village_id,
                target_village_id,
                player_id,
                army,
                target,
                attack_type,
                arrives_at,
                returns_at,
            },
        },
    )
}

pub(crate) fn scout_arrived_fact(workflow: &ScoutArrivalWorkflow) -> VillageEvent {
    VillageEvent::ScoutArrived {
        movement_id: workflow.movement_id,
        army_id: workflow.army_id,
        action_id: workflow.action_id,
        return_action_id: workflow.return_action_id,
        player_id: workflow.player_id,
        source_village_id: workflow.source_village_id,
        target_village_id: workflow.target_village_id,
        army: workflow.army.clone(),
        target: workflow.target.clone(),
        attack_type: workflow.attack_type.clone(),
        arrives_at: workflow.arrives_at,
        returns_at: workflow.returns_at,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn settlers_arrival_scheduled_action(
    action_id: Uuid,
    movement_id: Uuid,
    army_id: Uuid,
    village_id: u32,
    source_village_id: u32,
    target_village_id: u32,
    target_position: parabellum_types::map::Position,
    player_id: Uuid,
    village_name: String,
    tribe: parabellum_types::tribe::Tribe,
    arrives_at: chrono::DateTime<chrono::Utc>,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        action_id,
        arrives_at,
        ScheduledActionPayload::SettlersArrival {
            workflow: SettlersArrivalWorkflow {
                action_id,
                movement_id,
                army_id,
                village_id,
                source_village_id,
                target_village_id,
                target_position,
                player_id,
                village_name,
                tribe,
                arrives_at,
            },
        },
    )
}

pub(crate) fn settlers_foundation_facts(
    workflow: SettlersArrivalWorkflow,
    default_buildings: Vec<parabellum_game::models::village::VillageBuilding>,
) -> Vec<(u32, VillageEvent)> {
    vec![
        (
            workflow.source_village_id,
            VillageEvent::SettlersArrived {
                action_id: workflow.action_id,
                movement_id: workflow.movement_id,
                army_id: workflow.army_id,
                player_id: workflow.player_id,
                source_village_id: workflow.source_village_id,
                target_village_id: workflow.target_village_id,
                target_position: workflow.target_position.clone(),
                village_name: workflow.village_name.clone(),
                tribe: workflow.tribe.clone(),
                arrives_at: workflow.arrives_at,
            },
        ),
        (
            workflow.target_village_id,
            VillageEvent::VillageFounded {
                village_id: workflow.target_village_id,
                village_name: workflow.village_name,
                position: workflow.target_position,
                tribe: workflow.tribe,
                player_id: workflow.player_id,
                parent_village_id: Some(workflow.source_village_id),
                buildings: default_buildings,
            },
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn attack_arrival_scheduled_action(
    action_id: Uuid,
    movement_id: Uuid,
    army_id: Uuid,
    return_action_id: Uuid,
    village_id: u32,
    source_village_id: u32,
    target_village_id: u32,
    player_id: Uuid,
    army: Army,
    attack_type: parabellum_types::battle::AttackType,
    catapult_targets: [Option<parabellum_types::buildings::BuildingName>; 2],
    arrives_at: chrono::DateTime<chrono::Utc>,
    returns_at: chrono::DateTime<chrono::Utc>,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        action_id,
        arrives_at,
        ScheduledActionPayload::AttackArrival {
            workflow: AttackArrivalWorkflow {
                action_id,
                movement_id,
                army_id,
                return_action_id,
                village_id,
                source_village_id,
                target_village_id,
                player_id,
                army,
                attack_type,
                catapult_targets,
                arrives_at,
                returns_at,
            },
        },
    )
}

pub(crate) fn attack_arrived_fact(workflow: &AttackArrivalWorkflow) -> VillageEvent {
    VillageEvent::AttackArrived {
        movement_id: workflow.movement_id,
        army_id: workflow.army_id,
        action_id: workflow.action_id,
        return_action_id: workflow.return_action_id,
        player_id: workflow.player_id,
        source_village_id: workflow.source_village_id,
        target_village_id: workflow.target_village_id,
        army: workflow.army.clone(),
        attack_type: workflow.attack_type.clone(),
        catapult_targets: workflow.catapult_targets.clone(),
        arrives_at: workflow.arrives_at,
        returns_at: workflow.returns_at,
    }
}
