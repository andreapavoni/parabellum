use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    ArmyReturnWorkflow, AttackArrivalWorkflow, ReinforcementArrivalWorkflow, ScheduledAction,
    ScheduledActionPayload, ScoutArrivalWorkflow, SettlersArrivalWorkflow, VillageModel,
};
use parabellum_game::models::army::Army;
use parabellum_types::common::ResourceGroup;
use uuid::Uuid;

use crate::es::VillageEsService;

pub(crate) fn army_return_scheduled_action_from_workflow(
    action_id: Uuid,
    workflow: ArmyReturnWorkflow,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        action_id,
        workflow.returns_at,
        ScheduledActionPayload::ArmyReturn { workflow },
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn army_return_workflow(
    movement_id: Uuid,
    army_id: Uuid,
    village_id: u32,
    source_village_id: u32,
    target_village_id: u32,
    player_id: Uuid,
    army: Army,
    bounty: Option<ResourceGroup>,
    returns_at: chrono::DateTime<chrono::Utc>,
) -> ArmyReturnWorkflow {
    ArmyReturnWorkflow {
        village_id,
        movement_id,
        army_id,
        source_village_id,
        target_village_id,
        player_id,
        army,
        bounty,
        returns_at,
    }
}

pub(crate) fn army_return_events(
    action_id: Uuid,
    workflow: ArmyReturnWorkflow,
) -> super::WorkflowEvents {
    super::WorkflowEvents::one(
        workflow.source_village_id,
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
        },
    )
}

fn reinforcement_arrival_scheduled_action(
    workflow: ReinforcementArrivalWorkflow,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        workflow.movement_id,
        workflow.arrives_at,
        ScheduledActionPayload::ReinforcementArrival { workflow },
    )
}

fn reinforcement_arrival_workflow(
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

pub(crate) fn reinforcement_arrival_scheduled_action_from_event(
    event: &VillageEvent,
) -> Result<ScheduledAction, CqrsError> {
    let VillageEvent::ReinforcementSent {
        movement_id,
        army_id,
        player_id,
        source_village_id,
        target_village_id,
        army,
        arrives_at,
    } = event
    else {
        unreachable!(
            "reinforcement_arrival_scheduled_action_from_event called with non-ReinforcementSent event"
        );
    };

    reinforcement_arrival_scheduled_action(reinforcement_arrival_workflow(
        *movement_id,
        *army_id,
        *player_id,
        *source_village_id,
        *target_village_id,
        army.clone(),
        *arrives_at,
    ))
}

pub(crate) async fn reinforcement_arrival_events(
    svc: &VillageEsService,
    workflow: ReinforcementArrivalWorkflow,
) -> Result<super::WorkflowEvents, CqrsError> {
    let source = svc.get_village(workflow.source_village_id).await?;
    let target = svc.get_village(workflow.target_village_id).await?;
    Ok(reinforcement_arrival_events_from_models(
        workflow, &source, &target,
    ))
}

fn reinforcement_arrival_events_from_models(
    workflow: ReinforcementArrivalWorkflow,
    source: &VillageModel,
    target: &VillageModel,
) -> super::WorkflowEvents {
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

    super::WorkflowEvents::from_events(vec![
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
    ])
}

fn scout_arrival_scheduled_action(
    workflow: ScoutArrivalWorkflow,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        workflow.action_id,
        workflow.arrives_at,
        ScheduledActionPayload::ScoutArrival { workflow },
    )
}

pub(crate) fn scout_arrival_scheduled_action_from_event(
    event: &VillageEvent,
) -> Result<ScheduledAction, CqrsError> {
    let VillageEvent::ScoutSent {
        movement_id,
        army_id,
        arrival_action_id,
        return_action_id,
        player_id,
        source_village_id,
        target_village_id,
        army,
        target,
        attack_type,
        arrives_at,
        returns_at,
    } = event
    else {
        unreachable!("scout_arrival_scheduled_action_from_event called with non-ScoutSent event");
    };

    scout_arrival_scheduled_action(ScoutArrivalWorkflow {
        action_id: *arrival_action_id,
        movement_id: *movement_id,
        army_id: *army_id,
        return_action_id: *return_action_id,
        village_id: *source_village_id,
        source_village_id: *source_village_id,
        target_village_id: *target_village_id,
        player_id: *player_id,
        army: army.clone(),
        target: target.clone(),
        attack_type: attack_type.clone(),
        arrives_at: *arrives_at,
        returns_at: *returns_at,
    })
}

pub(crate) fn scout_arrived_events(workflow: &ScoutArrivalWorkflow) -> super::WorkflowEvents {
    super::WorkflowEvents::one(workflow.source_village_id, scout_arrived_fact(workflow))
}

fn settlers_arrival_scheduled_action(
    workflow: SettlersArrivalWorkflow,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        workflow.action_id,
        workflow.arrives_at,
        ScheduledActionPayload::SettlersArrival { workflow },
    )
}

pub(crate) fn settlers_arrival_scheduled_action_from_event(
    event: &VillageEvent,
) -> Result<ScheduledAction, CqrsError> {
    let VillageEvent::SettlersSent {
        action_id,
        movement_id,
        army_id,
        player_id,
        source_village_id,
        target_village_id,
        target_position,
        village_name,
        tribe,
        arrives_at,
        ..
    } = event
    else {
        unreachable!(
            "settlers_arrival_scheduled_action_from_event called with non-SettlersSent event"
        );
    };

    settlers_arrival_scheduled_action(SettlersArrivalWorkflow {
        action_id: *action_id,
        movement_id: *movement_id,
        army_id: *army_id,
        village_id: *source_village_id,
        source_village_id: *source_village_id,
        target_village_id: *target_village_id,
        target_position: target_position.clone(),
        player_id: *player_id,
        village_name: village_name.clone(),
        tribe: tribe.clone(),
        arrives_at: *arrives_at,
    })
}

fn attack_arrival_scheduled_action(
    workflow: AttackArrivalWorkflow,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        workflow.action_id,
        workflow.arrives_at,
        ScheduledActionPayload::AttackArrival { workflow },
    )
}

pub(crate) fn attack_arrival_scheduled_action_from_event(
    event: &VillageEvent,
) -> Result<ScheduledAction, CqrsError> {
    let VillageEvent::AttackArrivalScheduled {
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
    } = event
    else {
        unreachable!(
            "attack_arrival_scheduled_action_from_event called with non-AttackArrivalScheduled event"
        );
    };

    attack_arrival_scheduled_action(AttackArrivalWorkflow {
        action_id: *action_id,
        movement_id: *movement_id,
        army_id: *army_id,
        return_action_id: *return_action_id,
        village_id: *source_village_id,
        source_village_id: *source_village_id,
        target_village_id: *target_village_id,
        player_id: *player_id,
        army: army.clone(),
        attack_type: attack_type.clone(),
        catapult_targets: catapult_targets.clone(),
        arrives_at: *arrives_at,
        returns_at: *returns_at,
    })
}

pub(crate) fn attack_arrived_events(workflow: &AttackArrivalWorkflow) -> super::WorkflowEvents {
    super::WorkflowEvents::one(workflow.source_village_id, attack_arrived_fact(workflow))
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
