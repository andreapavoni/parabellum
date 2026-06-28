//! Queue read helpers backed by scheduled-action rows.

use parabellum_app::villages::models::{ScheduledActionPayload, ScheduledActionType};
use parabellum_app::villages::projection_repositories::{
    ScheduledActionFilter, ScheduledActionOrder,
};
use parabellum_app::villages::read_models::{
    AcademyQueueItem, BuildingQueueItem, SmithyQueueItem, TrainingQueueItem, TrapQueueItem,
    VillageQueues,
};
use parabellum_types::errors::{ApplicationError, DbError};

use super::{PostgresScheduledActionRepository, queries, rows::DbScheduledActionRow};

impl PostgresScheduledActionRepository {
    pub(crate) async fn list_village_queues(
        &self,
        village_id: u32,
    ) -> Result<VillageQueues, ApplicationError> {
        let rows = self.load_active_queue_rows(village_id).await?;
        map_rows_to_village_queues(rows)
    }

    async fn load_active_queue_rows(
        &self,
        village_id: u32,
    ) -> Result<Vec<DbScheduledActionRow>, ApplicationError> {
        let filter = ScheduledActionFilter::new()
            .village(village_id)
            .action_types(vec![
                ScheduledActionType::AddBuilding,
                ScheduledActionType::UpgradeBuilding,
                ScheduledActionType::DowngradeBuilding,
                ScheduledActionType::TrainUnit,
                ScheduledActionType::ResearchAcademy,
                ScheduledActionType::ResearchSmithy,
                ScheduledActionType::TrapBuild,
            ])
            .active()
            .order_by(ScheduledActionOrder::ExecuteAtAsc);

        queries::scheduled_action_row_query(filter)
            .build_query_as()
            .fetch_all(self.pool())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))
    }
}

fn map_rows_to_village_queues(
    rows: Vec<DbScheduledActionRow>,
) -> Result<VillageQueues, ApplicationError> {
    let mut queues = VillageQueues::default();
    for row in rows {
        let status = row.status.into();
        let payload: ScheduledActionPayload = serde_json::from_value(row.payload)
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        match payload {
            ScheduledActionPayload::Building { workflow } => {
                queues.building.push(BuildingQueueItem {
                    job_id: row.id,
                    kind: workflow.kind,
                    slot_id: workflow.slot_id,
                    building_name: workflow.building_name,
                    target_level: workflow.level,
                    status,
                    finishes_at: row.execute_at,
                });
            }
            ScheduledActionPayload::Training { workflow } => {
                queues.training.push(TrainingQueueItem {
                    job_id: row.id,
                    slot_id: workflow.slot_id,
                    unit: workflow.unit,
                    quantity: workflow.quantity_remaining,
                    time_per_unit: workflow.time_per_unit,
                    status,
                    finishes_at: row.execute_at,
                });
            }
            ScheduledActionPayload::Research { workflow } => match row.action_type.into() {
                ScheduledActionType::ResearchAcademy => queues.academy.push(AcademyQueueItem {
                    job_id: row.id,
                    unit: workflow.unit,
                    status,
                    finishes_at: row.execute_at,
                }),
                ScheduledActionType::ResearchSmithy => queues.smithy.push(SmithyQueueItem {
                    job_id: row.id,
                    unit: workflow.unit,
                    status,
                    finishes_at: row.execute_at,
                }),
                _ => {}
            },
            ScheduledActionPayload::TrapBuild { workflow } => {
                queues.traps.push(TrapQueueItem {
                    job_id: row.id,
                    quantity: workflow.quantity_remaining,
                    time_per_trap: workflow.time_per_trap,
                    status,
                    finishes_at: row.execute_at,
                });
            }
            _ => {}
        }
    }

    Ok(queues)
}
