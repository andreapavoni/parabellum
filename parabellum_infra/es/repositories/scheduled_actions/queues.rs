//! Queue read helpers backed by scheduled-action rows.

use chrono::{DateTime, Utc};
use parabellum_app::villages::models::{
    BuildingWorkflow, ResearchWorkflow, ResearchWorkflowKind, ScheduledActionPayload,
    ScheduledActionStatus, ScheduledActionType, TrainingWorkflow, TrapBuildWorkflow,
};
use parabellum_app::villages::projection_repositories::{
    ScheduledActionFilter, ScheduledActionOrder,
};
use parabellum_app::villages::read_models::{
    AcademyQueueItem, BuildingQueueItem, SmithyQueueItem, TrainingQueueItem, TrapQueueItem,
    VillageQueues,
};
use parabellum_types::errors::{ApplicationError, DbError};
use uuid::Uuid;

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
        append_queue_row(&mut queues, row)?;
    }

    Ok(queues)
}

fn append_queue_row(
    queues: &mut VillageQueues,
    row: DbScheduledActionRow,
) -> Result<(), ApplicationError> {
    let status = ScheduledActionStatus::from(row.status);
    let payload = decode_queue_payload(row.payload)?;

    match payload {
        ScheduledActionPayload::Building { workflow } => {
            queues
                .building
                .push(building_queue_item(row.id, row.execute_at, status, workflow));
        }
        ScheduledActionPayload::Training { workflow } => {
            queues
                .training
                .push(training_queue_item(row.id, row.execute_at, status, workflow));
        }
        ScheduledActionPayload::Research { workflow } => {
            append_research_queue_item(queues, row.id, row.execute_at, status, workflow);
        }
        ScheduledActionPayload::TrapBuild { workflow } => {
            queues
                .traps
                .push(trap_queue_item(row.id, row.execute_at, status, workflow));
        }
        _ => {}
    }

    Ok(())
}

fn decode_queue_payload(
    payload: serde_json::Value,
) -> Result<ScheduledActionPayload, ApplicationError> {
    serde_json::from_value(payload).map_err(|e| ApplicationError::Unknown(e.to_string()))
}

fn building_queue_item(
    job_id: Uuid,
    finishes_at: DateTime<Utc>,
    status: ScheduledActionStatus,
    workflow: BuildingWorkflow,
) -> BuildingQueueItem {
    BuildingQueueItem {
        job_id,
        kind: workflow.kind,
        slot_id: workflow.slot_id,
        building_name: workflow.building_name,
        target_level: workflow.level,
        status,
        finishes_at,
    }
}

fn training_queue_item(
    job_id: Uuid,
    finishes_at: DateTime<Utc>,
    status: ScheduledActionStatus,
    workflow: TrainingWorkflow,
) -> TrainingQueueItem {
    TrainingQueueItem {
        job_id,
        slot_id: workflow.slot_id,
        unit: workflow.unit,
        quantity: workflow.quantity_remaining,
        time_per_unit: workflow.time_per_unit,
        status,
        finishes_at,
    }
}

fn append_research_queue_item(
    queues: &mut VillageQueues,
    job_id: Uuid,
    finishes_at: DateTime<Utc>,
    status: ScheduledActionStatus,
    workflow: ResearchWorkflow,
) {
    match workflow.kind {
        ResearchWorkflowKind::Academy => queues.academy.push(AcademyQueueItem {
            job_id,
            unit: workflow.unit,
            status,
            finishes_at,
        }),
        ResearchWorkflowKind::Smithy => queues.smithy.push(SmithyQueueItem {
            job_id,
            unit: workflow.unit,
            status,
            finishes_at,
        }),
    }
}

fn trap_queue_item(
    job_id: Uuid,
    finishes_at: DateTime<Utc>,
    status: ScheduledActionStatus,
    workflow: TrapBuildWorkflow,
) -> TrapQueueItem {
    TrapQueueItem {
        job_id,
        quantity: workflow.quantity_remaining,
        time_per_trap: workflow.time_per_trap,
        status,
        finishes_at,
    }
}
