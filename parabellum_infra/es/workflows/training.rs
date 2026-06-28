use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{ScheduledAction, ScheduledActionPayload, TrainingWorkflow};
use parabellum_types::army::UnitName;
use parabellum_types::common::ResourceGroup;
use uuid::Uuid;

pub(crate) struct ScheduledTrainingAction {
    pub(crate) village_id: u32,
    pub(crate) action: ScheduledAction,
    pub(crate) cost: ResourceGroup,
}

fn training_scheduled_action(
    action_id: Uuid,
    execute_at: chrono::DateTime<chrono::Utc>,
    village_id: u32,
    player_id: Uuid,
    slot_id: u8,
    unit: UnitName,
    time_per_unit: i32,
    quantity_remaining: i32,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        action_id,
        execute_at,
        ScheduledActionPayload::Training {
            workflow: TrainingWorkflow {
                village_id,
                player_id,
                slot_id,
                unit,
                time_per_unit,
                quantity_remaining,
                execute_at,
            },
        },
    )
}

pub(crate) fn training_scheduled_action_from_event(
    event: &VillageEvent,
) -> Result<ScheduledTrainingAction, CqrsError> {
    let VillageEvent::UnitTrainingScheduled {
        action_id,
        player_id,
        village_id,
        slot_id,
        unit,
        time_per_unit,
        quantity_remaining,
        cost,
        execute_at,
    } = event
    else {
        unreachable!(
            "training_scheduled_action_from_event called with non-UnitTrainingScheduled event"
        );
    };

    Ok(ScheduledTrainingAction {
        village_id: *village_id,
        action: training_scheduled_action(
            *action_id,
            *execute_at,
            *village_id,
            *player_id,
            *slot_id,
            unit.clone(),
            *time_per_unit,
            *quantity_remaining,
        )?,
        cost: cost.clone(),
    })
}

pub(crate) fn completion_events(
    action_id: Uuid,
    workflow: TrainingWorkflow,
) -> super::WorkflowEvents {
    let TrainingWorkflow {
        village_id,
        player_id,
        slot_id,
        unit,
        time_per_unit,
        quantity_remaining,
        execute_at,
    } = workflow;

    if quantity_remaining <= 0 {
        return super::WorkflowEvents::new();
    }

    let mut events = super::WorkflowEvents::one(
        village_id,
        VillageEvent::UnitTrained {
            action_id,
            player_id,
            village_id,
            unit: unit.clone(),
            quantity_trained: 1,
        },
    );

    let remaining_after = quantity_remaining - 1;
    if remaining_after > 0 {
        events.push(
            village_id,
            VillageEvent::UnitTrainingScheduled {
                action_id: Uuid::new_v4(),
                player_id,
                village_id,
                slot_id,
                unit,
                time_per_unit,
                quantity_remaining: remaining_after,
                cost: parabellum_types::common::ResourceGroup::new(0, 0, 0, 0),
                execute_at: execute_at + chrono::Duration::seconds(time_per_unit.max(1) as i64),
            },
        );
    }

    events
}
