use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    BuildingWorkflow, BuildingWorkflowKind, ScheduledAction, ScheduledActionPayload,
};
use parabellum_types::buildings::BuildingName;
use uuid::Uuid;

pub(crate) fn scheduled_action(
    action_id: Uuid,
    execute_at: chrono::DateTime<chrono::Utc>,
    kind: BuildingWorkflowKind,
    village_id: u32,
    player_id: Uuid,
    slot_id: u8,
    building_name: BuildingName,
    level: u8,
    speed: i8,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        action_id,
        execute_at,
        ScheduledActionPayload::Building {
            workflow: BuildingWorkflow {
                kind,
                village_id,
                player_id,
                slot_id,
                building_name,
                level,
                speed,
            },
        },
    )
}

pub(crate) fn completion_fact(action_id: Uuid, workflow: BuildingWorkflow) -> VillageEvent {
    let BuildingWorkflow {
        kind,
        player_id,
        village_id,
        slot_id,
        building_name,
        level,
        speed,
    } = workflow;

    match kind {
        BuildingWorkflowKind::Add => VillageEvent::BuildingAdded {
            action_id,
            player_id,
            village_id,
            slot_id,
            building_name,
            level,
            speed,
        },
        BuildingWorkflowKind::Upgrade => VillageEvent::BuildingUpgraded {
            action_id,
            player_id,
            village_id,
            slot_id,
            building_name,
            level,
            speed,
        },
        BuildingWorkflowKind::Downgrade => VillageEvent::BuildingDowngraded {
            action_id,
            player_id,
            village_id,
            slot_id,
            building_name,
            level,
            speed,
        },
    }
}
