//! Building read helpers for `VillageEsService`.
//!
//! These helpers compose scheduled building workflow rows into the context
//! needed to cancel one building action and all later pending actions for the
//! same slot.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::projection_repositories::ScheduledActionRepository;
use parabellum_app::villages::CancelBuildingConstructionContext;
use parabellum_app::villages::{
    BuildingCancellationAction, BuildingCancellationPolicy,
    models::{ScheduledAction, ScheduledActionPayload, ScheduledActionType},
};

use crate::es::PostgresScheduledActionRepository;

use super::super::VillageEsService;

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

        BuildingCancellationPolicy {
            village_id,
            action_id,
            canceled_at,
            actions: decode_building_actions(actions)?,
        }
        .context()
        .map_err(CqrsError::domain_source)
    }
}

fn decode_building_actions(
    actions: Vec<ScheduledAction>,
) -> Result<Vec<BuildingCancellationAction>, CqrsError> {
    actions
        .into_iter()
        .map(|action| {
            let ScheduledActionPayload::Building { workflow } =
                action.payload().map_err(CqrsError::domain_source)?
            else {
                return Err(CqrsError::EventStore(
                    "Scheduled action is not a building workflow".to_string(),
                ));
            };
            Ok(BuildingCancellationAction {
                id: action.id,
                status: action.status,
                execute_at: action.execute_at,
                created_at: action.created_at.unwrap_or_else(chrono::Utc::now),
                workflow,
            })
        })
        .collect()
}
