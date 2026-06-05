use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{ResearchWorkflow, ScheduledAction, ScheduledActionPayload};
use parabellum_types::army::UnitName;
use parabellum_types::common::ResourceGroup;
use uuid::Uuid;

pub(crate) use parabellum_app::villages::models::ResearchWorkflowKind;

pub(crate) struct ScheduledResearchAction {
    pub(crate) village_id: u32,
    pub(crate) action: ScheduledAction,
    pub(crate) cost: ResourceGroup,
}

fn scheduled_action(
    action_id: Uuid,
    execute_at: chrono::DateTime<chrono::Utc>,
    kind: ResearchWorkflowKind,
    village_id: u32,
    player_id: Uuid,
    unit: UnitName,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        action_id,
        execute_at,
        ScheduledActionPayload::Research {
            workflow: ResearchWorkflow {
                kind,
                village_id,
                player_id,
                unit,
            },
        },
    )
}

pub(crate) fn scheduled_action_from_event(
    event: &VillageEvent,
) -> Result<ScheduledResearchAction, CqrsError> {
    let (kind, action_id, player_id, village_id, unit, cost, execute_at) = match event {
        VillageEvent::AcademyResearchScheduled {
            action_id,
            player_id,
            village_id,
            unit,
            cost,
            execute_at,
        } => (
            ResearchWorkflowKind::Academy,
            action_id,
            player_id,
            village_id,
            unit,
            cost,
            execute_at,
        ),
        VillageEvent::SmithyResearchScheduled {
            action_id,
            player_id,
            village_id,
            unit,
            cost,
            execute_at,
        } => (
            ResearchWorkflowKind::Smithy,
            action_id,
            player_id,
            village_id,
            unit,
            cost,
            execute_at,
        ),
        _ => unreachable!("scheduled_action_from_event called with non-research scheduled event"),
    };

    Ok(ScheduledResearchAction {
        village_id: *village_id,
        action: scheduled_action(
            *action_id,
            *execute_at,
            kind,
            *village_id,
            *player_id,
            unit.clone(),
        )?,
        cost: cost.clone(),
    })
}

pub(crate) fn completion_events(
    action_id: Uuid,
    workflow: ResearchWorkflow,
) -> super::WorkflowEvents {
    let ResearchWorkflow {
        kind,
        player_id,
        village_id,
        unit,
    } = workflow;

    let event = match kind {
        ResearchWorkflowKind::Academy => {
            parabellum_app::villages::VillageEvent::AcademyResearchCompleted {
                action_id,
                player_id,
                village_id,
                unit,
            }
        }
        ResearchWorkflowKind::Smithy => {
            parabellum_app::villages::VillageEvent::SmithyResearchCompleted {
                action_id,
                player_id,
                village_id,
                unit,
            }
        }
    };

    super::WorkflowEvents::one(village_id, event)
}
