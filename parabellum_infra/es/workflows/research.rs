use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{ResearchWorkflow, ScheduledAction, ScheduledActionPayload};
use parabellum_types::army::UnitName;
use uuid::Uuid;

pub(crate) use parabellum_app::villages::models::ResearchWorkflowKind;

pub(crate) fn scheduled_action(
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

pub(crate) fn completion_fact(action_id: Uuid, workflow: ResearchWorkflow) -> VillageEvent {
    let ResearchWorkflow {
        kind,
        player_id,
        village_id,
        unit,
    } = workflow;

    match kind {
        ResearchWorkflowKind::Academy => VillageEvent::AcademyResearchCompleted {
            action_id,
            player_id,
            village_id,
            unit,
        },
        ResearchWorkflowKind::Smithy => VillageEvent::SmithyResearchCompleted {
            action_id,
            player_id,
            village_id,
            unit,
        },
    }
}
