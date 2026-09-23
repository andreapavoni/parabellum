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

use crate::es::advisory_lock::AdvisoryLock;
use crate::es::lock_keys::SCHEDULED_ACTION_EXECUTION_LOCK_KEY;
use crate::es::workflows;
use crate::es::{
    CqrsError, PostgresArmyRepository, PostgresScheduledActionRepository, VillageEsService,
};
use parabellum_app::villages::models::{
    ScheduledAction, ScheduledActionPayload, ScheduledActionStatus,
};
use parabellum_app::villages::projection_repositories::{
    ArmyRepository, ScheduledActionRepository,
};

const SCHEDULED_ACTION_PROCESSING_STALE_AFTER_SECS: i64 = 120;

impl VillageEsService {
    /// Executes due scheduled actions by appending canonical workflow facts.
    ///
    /// Status transitions are persisted for each action (`completed` or `failed`).
    pub async fn process_due_actions(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<usize, CqrsError> {
        let Some(lock) =
            AdvisoryLock::try_acquire(self.pool(), SCHEDULED_ACTION_EXECUTION_LOCK_KEY).await?
        else {
            tracing::info!(
                action = "scheduler_skip_locked",
                "scheduled action execution lock is held; skipping tick"
            );
            return Ok(0);
        };

        let repo =
            PostgresScheduledActionRepository::new(crate::ProjectionDb::new(self.pool().clone()));
        let stale_before = before_or_equal
            - chrono::Duration::seconds(SCHEDULED_ACTION_PROCESSING_STALE_AFTER_SECS);
        let requeued = repo
            .requeue_stale_processing(stale_before)
            .await
            .map_err(CqrsError::domain_source)?;
        if requeued > 0 {
            tracing::warn!(
                action = "scheduler_requeue_stale_processing",
                requeued,
                stale_before = %stale_before,
                "requeued stale processing scheduled actions to pending"
            );
        }
        let actions = repo
            .take_due_pending(before_or_equal, limit)
            .await
            .map_err(CqrsError::domain_source)?;
        let claimed = actions.len();
        if claimed > 0 {
            tracing::info!(
                action = "scheduler_claim_due",
                claimed,
                limit,
                before_or_equal = %before_or_equal,
                "claimed due scheduled actions"
            );
        }

        let result = self.process_actions(&actions).await;
        lock.release().await?;
        result
    }

    pub async fn process_actions(&self, actions: &[ScheduledAction]) -> Result<usize, CqrsError> {
        let mut processed = 0usize;
        let repo =
            PostgresScheduledActionRepository::new(crate::ProjectionDb::new(self.pool().clone()));

        for action in actions {
            let result = execute_action(self, action).await;
            if let Err(err) = &result {
                let next_status = if is_retryable(err) {
                    ScheduledActionStatus::Pending
                } else {
                    ScheduledActionStatus::Failed
                };
                repo.finish_processing(action.id, next_status).await?;
            }
            if let Err(err) = &result {
                tracing::warn!(
                    action = "scheduler_action_failed",
                    action_id = %action.id,
                    action_type = ?action.action_type,
                    error = %err,
                    "scheduled action attempt failed"
                );
            } else {
                tracing::info!(
                    action = "scheduler_action_completed",
                    action_id = %action.id,
                    action_type = ?action.action_type,
                    "scheduled action completed"
                );
            }
            processed += 1;
        }
        Ok(processed)
    }
}

/// Executes one scheduled action payload by appending canonical workflow fact(s).
pub(super) async fn execute_action(
    svc: &VillageEsService,
    action: &parabellum_app::villages::models::ScheduledAction,
) -> Result<(), CqrsError> {
    tracing::debug!(
        action_id = %action.id,
        execute_at = %action.execute_at,
        action_type = ?action.action_type,
        "executing scheduled action"
    );
    match action.payload().map_err(CqrsError::domain_source)? {
        ScheduledActionPayload::ReinforcementArrival { workflow } => {
            svc.append_scheduled_workflow_events(
                action.id,
                workflows::movements::reinforcement_arrival_events(svc, workflow).await?,
            )
            .await?;
        }
        ScheduledActionPayload::SettlersArrival { workflow } => {
            svc.append_scheduled_workflow_events(
                action.id,
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
            svc.append_scheduled_workflow_events(
                action.id,
                workflows::WorkflowEvents::from_events(events),
            )
            .await?;
        }
        ScheduledActionPayload::ArmyReturn { workflow } => {
            svc.append_scheduled_workflow_events(
                action.id,
                workflows::movements::army_return_events(action.id, workflow),
            )
            .await?;
        }
        ScheduledActionPayload::ScoutArrival { workflow } => {
            let mut events = workflows::movements::scout_arrived_events(&workflow).into_inner();
            events.extend(
                workflows::battles::resolve_scout(svc, workflow)
                    .await?
                    .into_inner(),
            );
            svc.append_scheduled_workflow_events(
                action.id,
                workflows::WorkflowEvents::from_events(events),
            )
            .await?;
        }
        ScheduledActionPayload::MerchantsArrival { workflow } => {
            svc.append_scheduled_workflow_events(
                action.id,
                workflows::merchants::arrival_events(svc, action.id, workflow).await?,
            )
            .await?;
        }
        ScheduledActionPayload::MerchantsReturn { workflow } => {
            svc.append_scheduled_workflow_events(
                action.id,
                workflows::merchants::return_events(action.id, workflow),
            )
            .await?;
        }
        ScheduledActionPayload::Building { workflow } => {
            svc.append_scheduled_workflow_events(
                action.id,
                workflows::buildings::completion_events(action.id, workflow),
            )
            .await?;
        }
        ScheduledActionPayload::Training { workflow } => {
            let workflow_events = workflows::training::completion_events(action.id, workflow);
            svc.append_scheduled_workflow_events(action.id, workflow_events)
                .await?;
        }
        ScheduledActionPayload::Research { workflow } => {
            svc.append_scheduled_workflow_events(
                action.id,
                workflows::research::completion_events(action.id, workflow),
            )
            .await?;
        }
        ScheduledActionPayload::HeroRevival { workflow } => {
            svc.append_scheduled_workflow_events(
                action.id,
                workflows::heroes::revival_events(svc, action.id, workflow).await?,
            )
            .await?;
        }
        ScheduledActionPayload::TrapBuild { workflow } => {
            let village = svc.get_village(workflow.village_id).await?;
            let mut trapper = parabellum_game::models::trapper::Trapper::from_buildings(
                &village.buildings,
                village.trapper,
                PostgresArmyRepository::new(crate::ProjectionDb::new(svc.pool().clone()))
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
            svc.append_scheduled_workflow_events(action.id, workflow_events)
                .await?;
        }
    }
    Ok(())
}

/// Preserve business failures while retrying optimistic conflicts and transient
/// SQL failures with the repository's persistent attempt budget.
fn is_retryable(error: &CqrsError) -> bool {
    if matches!(error, CqrsError::Conflict { .. }) {
        return true;
    }
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(error) = source {
        use parabellum_types::errors::{ApplicationError, DbError};
        // Transparent wrappers may forward source() past the SQLx enum itself.
        let sql_error = error
            .downcast_ref::<sqlx::Error>()
            .or_else(|| match error.downcast_ref::<DbError>() {
                Some(DbError::Database(error)) => Some(error),
                _ => None,
            })
            .or_else(|| match error.downcast_ref::<ApplicationError>() {
                Some(ApplicationError::Db(DbError::Database(error))) => Some(error),
                _ => None,
            });
        if let Some(error) = sql_error {
            return match error {
                sqlx::Error::Io(_) | sqlx::Error::PoolTimedOut | sqlx::Error::WorkerCrashed => true,
                sqlx::Error::Database(error) => error.code().is_some_and(|code| {
                    code.starts_with("08")
                        || matches!(
                            code.as_ref(),
                            "40001" | "40P01" | "55P03" | "57014" | "57P01" | "57P02" | "57P03"
                        )
                }),
                _ => false,
            };
        }
        source = error.source();
    }
    false
}
