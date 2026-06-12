//! Scheduled-action payload executor for `VillageEsService`.
//!
//! Each payload variant is executed as deterministic workflow progression.
//! Validation is assumed to have happened at scheduling time; this layer executes
//! payload intent and applies terminal status (`completed`/`failed`) upstream.
//!
//! Workflow modules:
//! - Keep branch logic in `execute_action` thin by delegating each payload
//!   variant to one focused `workflows::*` module.
//! - Pure workflows return deterministic completion facts without I/O.
//! - Async workflows may load read models or domain state before producing
//!   facts, but infrastructure remains orchestration-only.
//! - New scheduled workflows should follow the same shape: decode payload,
//!   delegate to a workflow module, append the returned `WorkflowEvents`.

use crate::es::workflows;
use crate::es::{CqrsError, PostgresArmyRepository, VillageEsService};
use parabellum_app::villages::repositories::ArmyRepository;
use parabellum_app::villages::VillageService;
use parabellum_app::villages::models::ScheduledActionPayload;

/// Executes one scheduled action payload by appending canonical workflow fact(s).
pub(super) async fn execute_action(
    svc: &VillageEsService,
    _service: &VillageService<'_, crate::es::VillageCqrsRuntime>,
    action: &parabellum_app::villages::models::ScheduledAction,
) -> Result<(), CqrsError> {
    tracing::debug!(
        action_id = %action.id,
        execute_at = %action.execute_at,
        action_type = ?action.action_type,
        "executing scheduled action"
    );
    let payload: ScheduledActionPayload =
        serde_json::from_value(action.payload.clone()).map_err(CqrsError::Serialization)?;
    match payload {
        ScheduledActionPayload::ReinforcementArrival { workflow } => {
            svc.append_workflow_events(
                workflows::movements::reinforcement_arrival_events(svc, workflow).await?,
            )
            .await?;
        }
        ScheduledActionPayload::SettlersArrival { workflow } => {
            svc.append_workflow_events(
                workflows::foundation::settlers_arrival_events(svc, workflow).await?,
            )
            .await?;
        }
        ScheduledActionPayload::AttackArrival { workflow } => {
            let mut events = workflows::movements::attack_arrived_events(&workflow).into_inner();
            events.extend(
                workflows::battles::resolve_attack(svc, workflow)
                    .await?
                    .into_inner(),
            );
            svc.append_workflow_events(workflows::WorkflowEvents::from_events(events))
                .await?;
        }
        ScheduledActionPayload::ArmyReturn { workflow } => {
            svc.append_workflow_events(workflows::movements::army_return_events(
                action.id, workflow,
            ))
            .await?;
        }
        ScheduledActionPayload::ScoutArrival { workflow } => {
            let mut events = workflows::movements::scout_arrived_events(&workflow).into_inner();
            events.extend(
                workflows::battles::resolve_scout(svc, workflow)
                    .await?
                    .into_inner(),
            );
            svc.append_workflow_events(workflows::WorkflowEvents::from_events(events))
                .await?;
        }
        ScheduledActionPayload::MerchantsArrival { workflow } => {
            svc.append_workflow_events(
                workflows::merchants::arrival_events(svc, action.id, workflow).await?,
            )
            .await?;
        }
        ScheduledActionPayload::MerchantsReturn { workflow } => {
            svc.append_workflow_events(workflows::merchants::return_events(action.id, workflow))
                .await?;
        }
        ScheduledActionPayload::Building { workflow } => {
            svc.append_workflow_events(workflows::buildings::completion_events(
                action.id, workflow,
            ))
            .await?;
        }
        ScheduledActionPayload::Training { workflow } => {
            let workflow_events = workflows::training::completion_events(action.id, workflow);
            svc.append_workflow_events(workflow_events).await?;
        }
        ScheduledActionPayload::Research { workflow } => {
            svc.append_workflow_events(workflows::research::completion_events(action.id, workflow))
                .await?;
        }
        ScheduledActionPayload::HeroRevival { workflow } => {
            svc.append_workflow_events(
                workflows::heroes::revival_events(svc, action.id, workflow).await?,
            )
            .await?;
        }
        ScheduledActionPayload::TrapBuild { workflow } => {
            let village = svc.get_village(workflow.village_id).await?;
            let mut trapper = parabellum_game::models::trapper::Trapper::from_buildings(
                &village.buildings,
                village.trapper,
                PostgresArmyRepository::new(svc.pool().clone())
                    .army_context_for_village(workflow.village_id)
                    .await
                    .map_err(CqrsError::domain_source)?
                    .trapped_here
                    .iter()
                    .map(|army| army.units().immensity())
                    .sum(),
            );
            trapper.complete_trap_build(1);
            let workflow_events =
                workflows::traps::completion_events(action.id, workflow, trapper.state());
            svc.append_workflow_events(workflow_events).await?;
        }
    }
    Ok(())
}
