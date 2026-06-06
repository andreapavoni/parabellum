use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    BuildingWorkflow, BuildingWorkflowKind, ScheduledAction, ScheduledActionPayload,
};
use parabellum_types::buildings::BuildingName;
use parabellum_types::common::ResourceGroup;
use uuid::Uuid;

pub(crate) struct ScheduledBuildingAction {
    pub(crate) village_id: u32,
    pub(crate) action: ScheduledAction,
    pub(crate) cost: Option<ResourceGroup>,
}

fn scheduled_action(
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

pub(crate) fn scheduled_action_from_event(
    event: &VillageEvent,
) -> Result<ScheduledBuildingAction, CqrsError> {
    let (
        kind,
        action_id,
        player_id,
        village_id,
        slot_id,
        building_name,
        level,
        speed,
        execute_at,
        cost,
    ) = match event {
        VillageEvent::BuildingConstructionScheduled {
            action_id,
            player_id,
            village_id,
            slot_id,
            building_name,
            level,
            speed,
            cost,
            execute_at,
        } => (
            BuildingWorkflowKind::Add,
            action_id,
            player_id,
            village_id,
            slot_id,
            building_name,
            level,
            speed,
            execute_at,
            Some(cost.clone()),
        ),
        VillageEvent::BuildingUpgradeScheduled {
            action_id,
            player_id,
            village_id,
            slot_id,
            building_name,
            level,
            speed,
            cost,
            execute_at,
        } => (
            BuildingWorkflowKind::Upgrade,
            action_id,
            player_id,
            village_id,
            slot_id,
            building_name,
            level,
            speed,
            execute_at,
            Some(cost.clone()),
        ),
        VillageEvent::BuildingDowngradeScheduled {
            action_id,
            player_id,
            village_id,
            slot_id,
            building_name,
            level,
            speed,
            execute_at,
        } => (
            BuildingWorkflowKind::Downgrade,
            action_id,
            player_id,
            village_id,
            slot_id,
            building_name,
            level,
            speed,
            execute_at,
            None,
        ),
        _ => unreachable!("scheduled_action_from_event called with non-building scheduled event"),
    };

    Ok(ScheduledBuildingAction {
        village_id: *village_id,
        action: scheduled_action(
            *action_id,
            *execute_at,
            kind,
            *village_id,
            *player_id,
            *slot_id,
            building_name.clone(),
            *level,
            *speed,
        )?,
        cost,
    })
}

pub(crate) fn completion_events(
    action_id: Uuid,
    workflow: BuildingWorkflow,
) -> super::WorkflowEvents {
    let BuildingWorkflow {
        kind,
        player_id,
        village_id,
        slot_id,
        building_name,
        level,
        speed,
    } = workflow;

    let event = match kind {
        BuildingWorkflowKind::Add => parabellum_app::villages::VillageEvent::BuildingAdded {
            action_id,
            player_id,
            village_id,
            slot_id,
            building_name,
            level,
            speed,
        },
        BuildingWorkflowKind::Upgrade => parabellum_app::villages::VillageEvent::BuildingUpgraded {
            action_id,
            player_id,
            village_id,
            slot_id,
            building_name,
            level,
            speed,
        },
        BuildingWorkflowKind::Downgrade => {
            parabellum_app::villages::VillageEvent::BuildingDowngraded {
                action_id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
            }
        }
    };

    super::WorkflowEvents::one(village_id, event)
}
