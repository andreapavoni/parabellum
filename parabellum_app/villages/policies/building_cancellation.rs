//! Building cancellation policy.
//!
//! This policy selects the queued building workflows canceled by one
//! cancellation request and calculates the refund owed for remaining work.
//! Loading scheduled action rows is infrastructure; deciding which queued
//! workflows are canceled together is application policy.

use chrono::{DateTime, Utc};
use parabellum_game::models::buildings::Building;
use parabellum_types::{common::ResourceGroup, errors::GameError};
use uuid::Uuid;

use crate::villages::{
    models::{BuildingWorkflow, BuildingWorkflowKind, ScheduledActionStatus},
    ports::building_reads::CancelBuildingConstructionContext,
};

/// Queued building workflow considered by the cancellation policy.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildingCancellationAction {
    pub id: Uuid,
    pub status: ScheduledActionStatus,
    pub execute_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub workflow: BuildingWorkflow,
}

/// Cancellation request evaluated against currently active building workflows.
#[derive(Debug, Clone)]
pub struct BuildingCancellationPolicy {
    pub village_id: u32,
    pub action_id: Uuid,
    pub canceled_at: DateTime<Utc>,
    pub actions: Vec<BuildingCancellationAction>,
}

impl BuildingCancellationPolicy {
    /// Builds the command context for canceling a queued building workflow.
    ///
    /// Canceling one queued action also cancels later queued actions for the
    /// same slot because those later actions depend on the selected action.
    pub fn context(self) -> Result<CancelBuildingConstructionContext, GameError> {
        let Some(anchor) = self
            .actions
            .iter()
            .find(|action| action.id == self.action_id)
            .cloned()
        else {
            return Err(GameError::BuildingConstructionNotCancelable);
        };
        if anchor.status != ScheduledActionStatus::Pending
            || anchor.workflow.village_id != self.village_id
        {
            return Err(GameError::BuildingConstructionNotCancelable);
        }

        let mut actions = self.actions;
        actions.sort_by_key(|action| action.execute_at);

        let mut previous_execute_at = None;
        let mut action_ids = Vec::new();
        let mut refund = ResourceGroup::new(0, 0, 0, 0);

        for action in actions
            .into_iter()
            .filter(|action| action.workflow.slot_id == anchor.workflow.slot_id)
        {
            let started_at = previous_execute_at.unwrap_or(action.created_at);
            previous_execute_at = Some(action.execute_at);

            if action.execute_at < anchor.execute_at {
                continue;
            }
            if action.status != ScheduledActionStatus::Pending {
                return Err(GameError::BuildingConstructionNotCancelable);
            }

            action_ids.push(action.id);
            refund = add_resources(
                refund,
                prorated_building_refund(
                    &action.workflow,
                    started_at,
                    action.execute_at,
                    self.canceled_at,
                )?,
            );
        }

        if action_ids.is_empty() {
            return Err(GameError::BuildingConstructionNotCancelable);
        }

        Ok(CancelBuildingConstructionContext {
            action_ids,
            player_id: anchor.workflow.player_id,
            village_id: anchor.workflow.village_id,
            execute_at: anchor.execute_at,
            refund,
        })
    }
}

fn prorated_building_refund(
    workflow: &BuildingWorkflow,
    started_at: DateTime<Utc>,
    execute_at: DateTime<Utc>,
    canceled_at: DateTime<Utc>,
) -> Result<ResourceGroup, GameError> {
    let cost = match workflow.kind {
        BuildingWorkflowKind::Add | BuildingWorkflowKind::Upgrade => {
            Building::new(workflow.building_name.clone(), workflow.speed)
                .at_level(workflow.level, workflow.speed)?
                .cost()
                .resources
        }
        BuildingWorkflowKind::Downgrade => ResourceGroup::new(0, 0, 0, 0),
    };
    if cost.total() == 0 {
        return Ok(cost);
    }

    let total_secs = (execute_at - started_at).num_seconds().max(0) as u64;
    if total_secs == 0 {
        return Ok(ResourceGroup::new(0, 0, 0, 0));
    }

    let elapsed_secs = (canceled_at - started_at)
        .num_seconds()
        .clamp(0, total_secs as i64) as u64;
    let remaining_secs = total_secs.saturating_sub(elapsed_secs);

    Ok(ResourceGroup::new(
        prorate_resource(cost.lumber(), remaining_secs, total_secs),
        prorate_resource(cost.clay(), remaining_secs, total_secs),
        prorate_resource(cost.iron(), remaining_secs, total_secs),
        prorate_resource(cost.crop(), remaining_secs, total_secs),
    ))
}

fn prorate_resource(value: u32, remaining_secs: u64, total_secs: u64) -> u32 {
    ((value as u64 * remaining_secs) / total_secs) as u32
}

fn add_resources(left: ResourceGroup, right: ResourceGroup) -> ResourceGroup {
    ResourceGroup::new(
        left.lumber().saturating_add(right.lumber()),
        left.clay().saturating_add(right.clay()),
        left.iron().saturating_add(right.iron()),
        left.crop().saturating_add(right.crop()),
    )
}

#[cfg(test)]
mod tests {
    use parabellum_types::buildings::BuildingName;

    use super::*;

    fn workflow(
        kind: BuildingWorkflowKind,
        village_id: u32,
        player_id: Uuid,
        slot_id: u8,
        level: u8,
    ) -> BuildingWorkflow {
        BuildingWorkflow {
            kind,
            village_id,
            player_id,
            slot_id,
            building_name: BuildingName::Woodcutter,
            level,
            speed: 1,
        }
    }

    fn action(
        id: Uuid,
        workflow: BuildingWorkflow,
        created_at: DateTime<Utc>,
        execute_at: DateTime<Utc>,
    ) -> BuildingCancellationAction {
        BuildingCancellationAction {
            id,
            status: ScheduledActionStatus::Pending,
            created_at,
            execute_at,
            workflow,
        }
    }

    #[test]
    fn cancels_selected_action_and_later_actions_on_the_same_slot() {
        let player_id = Uuid::new_v4();
        let selected_id = Uuid::new_v4();
        let later_id = Uuid::new_v4();
        let other_slot_id = Uuid::new_v4();
        let started_at = Utc::now();
        let selected_at = started_at + chrono::Duration::seconds(100);
        let later_at = selected_at + chrono::Duration::seconds(100);

        let context = BuildingCancellationPolicy {
            village_id: 10,
            action_id: selected_id,
            canceled_at: started_at,
            actions: vec![
                action(
                    selected_id,
                    workflow(BuildingWorkflowKind::Upgrade, 10, player_id, 3, 2),
                    started_at,
                    selected_at,
                ),
                action(
                    later_id,
                    workflow(BuildingWorkflowKind::Upgrade, 10, player_id, 3, 3),
                    selected_at,
                    later_at,
                ),
                action(
                    other_slot_id,
                    workflow(BuildingWorkflowKind::Upgrade, 10, player_id, 4, 2),
                    started_at,
                    selected_at,
                ),
            ],
        }
        .context()
        .unwrap();

        assert_eq!(context.action_ids, vec![selected_id, later_id]);
        assert_eq!(context.player_id, player_id);
        assert_eq!(context.village_id, 10);
        assert!(context.refund.total() > 0);
    }

    #[test]
    fn refuses_non_pending_dependent_action() {
        let player_id = Uuid::new_v4();
        let selected_id = Uuid::new_v4();
        let later_id = Uuid::new_v4();
        let started_at = Utc::now();
        let selected_at = started_at + chrono::Duration::seconds(100);
        let later_at = selected_at + chrono::Duration::seconds(100);
        let mut later = action(
            later_id,
            workflow(BuildingWorkflowKind::Upgrade, 10, player_id, 3, 3),
            selected_at,
            later_at,
        );
        later.status = ScheduledActionStatus::Processing;

        let result = BuildingCancellationPolicy {
            village_id: 10,
            action_id: selected_id,
            canceled_at: started_at,
            actions: vec![
                action(
                    selected_id,
                    workflow(BuildingWorkflowKind::Upgrade, 10, player_id, 3, 2),
                    started_at,
                    selected_at,
                ),
                later,
            ],
        }
        .context();

        assert_eq!(
            result.unwrap_err(),
            GameError::BuildingConstructionNotCancelable
        );
    }
}
