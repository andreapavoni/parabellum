use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{ScheduledAction, ScheduledActionPayload, TrainingWorkflow};
use parabellum_types::army::UnitName;
use uuid::Uuid;

pub(crate) fn scheduled_action(
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

pub(crate) fn completion_facts(
    action_id: Uuid,
    workflow: TrainingWorkflow,
) -> Vec<(u32, VillageEvent)> {
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
        return vec![];
    }

    let mut events = vec![(
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
        events.push((
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
        ));
    }

    events
}
