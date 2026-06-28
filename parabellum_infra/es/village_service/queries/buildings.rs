//! Building read helpers for `VillageEsService`.
//!
//! These helpers compose scheduled building workflow rows into the context
//! needed to cancel one building action and all later pending actions for the
//! same slot.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::models::{
    BuildingWorkflow, BuildingWorkflowKind, ScheduledAction, ScheduledActionPayload,
    ScheduledActionStatus, ScheduledActionType,
};
use parabellum_app::villages::projection_repositories::ScheduledActionRepository;
use parabellum_game::models::buildings::Building;
use parabellum_types::common::ResourceGroup;
use parabellum_types::errors::{DbError, GameError};

use crate::es::PostgresScheduledActionRepository;

use super::super::{CancelBuildingConstructionContext, VillageEsService};

#[derive(Clone)]
struct CancelableBuildingAction {
    id: uuid::Uuid,
    status: ScheduledActionStatus,
    execute_at: chrono::DateTime<chrono::Utc>,
    created_at: chrono::DateTime<chrono::Utc>,
    workflow: BuildingWorkflow,
}

impl VillageEsService {
    /// Returns the cancellation context for a pending building construction action.
    pub async fn find_cancel_building_construction_context(
        &self,
        village_id: u32,
        action_id: uuid::Uuid,
        canceled_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<CancelBuildingConstructionContext, CqrsError> {
        let repo =
            PostgresScheduledActionRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        let mut actions = Vec::new();
        for action_type in [
            ScheduledActionType::AddBuilding,
            ScheduledActionType::UpgradeBuilding,
            ScheduledActionType::DowngradeBuilding,
        ] {
            actions.extend(
                repo.list_active_by_village_and_type(village_id, action_type)
                    .await
                    .map_err(CqrsError::domain_source)?,
            );
        }

        let mut building_actions = decode_building_actions(actions)?;
        let Some(anchor) = building_actions
            .iter()
            .find(|action| action.id == action_id)
            .cloned()
        else {
            return Err(CqrsError::EventStore(
                DbError::JobNotFound(action_id).to_string(),
            ));
        };
        if anchor.status != ScheduledActionStatus::Pending
            || anchor.workflow.village_id != village_id
        {
            return Err(CqrsError::domain_source(
                GameError::BuildingConstructionNotCancelable,
            ));
        }

        building_actions.sort_by_key(|action| action.execute_at);
        let mut previous_execute_at = None;
        let mut action_ids = Vec::new();
        let mut refund = ResourceGroup::new(0, 0, 0, 0);

        for action in building_actions
            .into_iter()
            .filter(|action| action.workflow.slot_id == anchor.workflow.slot_id)
        {
            let started_at = previous_execute_at.unwrap_or(action.created_at);
            previous_execute_at = Some(action.execute_at);

            if action.execute_at < anchor.execute_at {
                continue;
            }
            if action.status != ScheduledActionStatus::Pending {
                return Err(CqrsError::domain_source(
                    GameError::BuildingConstructionNotCancelable,
                ));
            }

            action_ids.push(action.id);
            refund = add_resources(
                refund,
                prorated_building_refund(
                    &action.workflow,
                    started_at,
                    action.execute_at,
                    canceled_at,
                )
                .map_err(CqrsError::domain_source)?,
            );
        }

        if action_ids.is_empty() {
            return Err(CqrsError::domain_source(
                GameError::BuildingConstructionNotCancelable,
            ));
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

fn decode_building_actions(
    actions: Vec<ScheduledAction>,
) -> Result<Vec<CancelableBuildingAction>, CqrsError> {
    actions
        .into_iter()
        .map(|action| {
            let payload: ScheduledActionPayload =
                serde_json::from_value(action.payload).map_err(CqrsError::Serialization)?;
            let ScheduledActionPayload::Building { workflow } = payload else {
                return Err(CqrsError::EventStore(
                    "Scheduled action is not a building workflow".to_string(),
                ));
            };
            Ok(CancelableBuildingAction {
                id: action.id,
                status: action.status,
                execute_at: action.execute_at,
                created_at: action.created_at.unwrap_or_else(chrono::Utc::now),
                workflow,
            })
        })
        .collect()
}

fn prorated_building_refund(
    workflow: &BuildingWorkflow,
    started_at: chrono::DateTime<chrono::Utc>,
    execute_at: chrono::DateTime<chrono::Utc>,
    canceled_at: chrono::DateTime<chrono::Utc>,
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
