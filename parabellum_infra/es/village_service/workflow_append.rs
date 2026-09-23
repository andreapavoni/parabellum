//! Atomic cross-stream workflow persistence, including scheduled-action completion.
use mini_cqrs_es::{Aggregate, AggregateSnapshot, CqrsError, EventMetadata, NewEvent};
use parabellum_app::villages::models::ScheduledActionStatus;
use parabellum_app::villages::{VillageAggregate, VillageEvent};
use uuid::Uuid;

use super::VillageEsService;
use crate::es::{
    PostgresEventStore, PostgresScheduledActionRepository, PostgresSnapshotStore, ReportProjector,
    VillageProjector, WorkflowStreamAppend, workflows,
};

impl VillageEsService {
    async fn append_village_workflow_events(
        &self,
        workflow_events: Vec<(u32, VillageEvent)>,
        action_id: Option<Uuid>,
    ) -> Result<(), CqrsError> {
        let mut tx = self.pool.begin().await.map_err(CqrsError::domain_source)?;
        sqlx::query("LOCK TABLE es_events IN ROW EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await
            .map_err(CqrsError::domain_source)?;
        let actions =
            PostgresScheduledActionRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        if let Some(id) = action_id {
            // Lock and check persisted ownership before any effects. A repeated
            // delivery after commit must not append another completion/continuation.
            if !actions.lock_processing_in_tx(&mut tx, id).await? {
                return Ok(());
            }
        }

        let mut grouped: Vec<(u32, Vec<NewEvent>)> = Vec::new();
        for (id, payload) in workflow_events {
            let event = NewEvent::from_payload(payload, EventMetadata::default())?;
            if let Some((_, events)) = grouped.iter_mut().find(|(stream_id, _)| *stream_id == id) {
                events.push(event);
            } else {
                grouped.push((id, vec![event]));
            }
        }
        let aggregate_type = std::any::type_name::<VillageAggregate>();
        let store = PostgresEventStore::new(crate::EventStoreDb::new(self.pool.clone()));
        let snapshots = PostgresSnapshotStore::new(crate::EventStoreDb::new(self.pool.clone()));
        let mut aggregates = Vec::with_capacity(grouped.len());
        let mut streams = Vec::with_capacity(grouped.len());
        for (id, events) in grouped {
            let aggregate = store
                .load_aggregate_in_tx::<VillageAggregate>(&mut tx, &id)
                .await?;
            streams.push(WorkflowStreamAppend {
                aggregate_id: id.to_string(),
                expected_version: aggregate.version(),
                events,
            });
            aggregates.push(aggregate);
        }
        let mut stored = store
            .append_workflow_events_in_tx(&mut tx, aggregate_type, &streams)
            .await?;
        stored.sort_by_key(|event| event.global_sequence.unwrap_or(i64::MAX));
        let villages = VillageProjector::new(self.pool.clone());
        let reports = ReportProjector::new(self.pool.clone());
        for event in &stored {
            villages.process_in_tx(&mut tx, event).await?;
            reports.process_in_tx(&mut tx, event).await?;
        }
        for (stream, aggregate) in streams.iter().zip(&mut aggregates) {
            for event in stored
                .iter()
                .filter(|event| event.aggregate_id == stream.aggregate_id)
            {
                aggregate.apply_events(std::slice::from_ref(event)).await?;
                aggregate.set_version(event.version);
            }
            snapshots
                .save_in_tx(
                    &mut tx,
                    AggregateSnapshot::new(aggregate, Some(aggregate.version()))?,
                )
                .await?;
        }
        if let Some(id) = action_id {
            actions
                .update_status_in_tx(&mut tx, id, ScheduledActionStatus::Completed)
                .await
                .map_err(CqrsError::domain_source)?;
        }
        tx.commit().await.map_err(CqrsError::domain_source)
    }

    pub(super) async fn append_workflow_events(
        &self,
        events: workflows::WorkflowEvents,
    ) -> Result<(), CqrsError> {
        if events.is_empty() {
            return Ok(());
        }
        self.append_village_workflow_events(events.into_inner(), None)
            .await
    }

    pub(super) async fn append_scheduled_workflow_events(
        &self,
        action_id: Uuid,
        events: workflows::WorkflowEvents,
    ) -> Result<(), CqrsError> {
        self.append_village_workflow_events(events.into_inner(), Some(action_id))
            .await
    }
}
