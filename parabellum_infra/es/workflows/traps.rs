use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{ScheduledAction, ScheduledActionPayload, TrapBuildWorkflow};
use parabellum_game::models::trapper::TrapperState;
use uuid::Uuid;

pub(crate) fn scheduled_action_from_event(event: &VillageEvent) -> Result<ScheduledAction, CqrsError> {
    let VillageEvent::TrapBuildScheduled {
        action_id,
        player_id,
        village_id,
        quantity_remaining,
        time_per_trap,
        execute_at,
        ..
    } = event
    else {
        unreachable!("scheduled_action_from_event called with non-TrapBuildScheduled event");
    };

    super::scheduled_action(
        *action_id,
        *execute_at,
        ScheduledActionPayload::TrapBuild {
            workflow: TrapBuildWorkflow {
                village_id: *village_id,
                player_id: *player_id,
                quantity_remaining: *quantity_remaining,
                time_per_trap: *time_per_trap,
                execute_at: *execute_at,
            },
        },
    )
}

pub(crate) fn completion_events(
    action_id: Uuid,
    workflow: TrapBuildWorkflow,
    trapper: TrapperState,
) -> super::WorkflowEvents {
    let TrapBuildWorkflow {
        village_id,
        player_id,
        quantity_remaining,
        time_per_trap,
        execute_at,
    } = workflow;

    if quantity_remaining <= 0 {
        return super::WorkflowEvents::new();
    }

    let mut events = super::WorkflowEvents::one(
        village_id,
        VillageEvent::TrapBuilt {
            action_id,
            player_id,
            village_id,
            quantity_built: 1,
            trapper,
        },
    );

    let remaining_after = quantity_remaining - 1;
    if remaining_after > 0 {
        events.push(
            village_id,
            VillageEvent::TrapBuildScheduled {
                action_id: Uuid::new_v4(),
                player_id,
                village_id,
                quantity_remaining: remaining_after,
                time_per_trap,
                cost: parabellum_types::common::ResourceGroup::new(0, 0, 0, 0),
                trapper,
                execute_at: execute_at + chrono::Duration::seconds(time_per_trap.max(1) as i64),
            },
        );
    }

    events
}
