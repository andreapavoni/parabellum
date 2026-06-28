//! Cross-stream workflow append mechanics for `VillageEsService`.

use mini_cqrs_es::anyhow::Result;
use mini_cqrs_es::{
    Aggregate, AggregateSnapshot, CqrsError, EventMetadata, EventPayload, EventStore, NewEvent,
    SnapshotStore,
};

use crate::es::workflows;
use crate::es::{
    PostgresEventStore, PostgresSnapshotStore, ReportProjector, VillageProjector,
    WorkflowStreamAppend,
};

use super::VillageEsService;

impl VillageEsService {
    /// Converts unordered `(village_id, event)` facts into stream-grouped append
    /// units with expected versions.
    ///
    /// Contract:
    /// - grouping is by aggregate stream id (`village_id`)
    /// - event order is preserved inside each stream group
    /// - expected versions are loaded immediately before append preparation
    async fn build_village_workflow_appends(
        &self,
        workflow_events: Vec<(u32, parabellum_app::villages::VillageEvent)>,
    ) -> Result<Vec<WorkflowStreamAppend>, CqrsError> {
        let aggregate_type = std::any::type_name::<parabellum_app::villages::VillageAggregate>();
        let store = PostgresEventStore::new(crate::EventStoreDb::new(self.pool.clone()));
        let mut grouped: Vec<(u32, Vec<NewEvent>)> = Vec::new();
        for (aggregate_id, payload) in workflow_events {
            let event = NewEvent {
                event_type: payload.name(),
                payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                metadata: EventMetadata::default(),
                timestamp: chrono::Utc::now(),
            };
            if let Some((_, events)) = grouped.iter_mut().find(|(id, _)| *id == aggregate_id) {
                events.push(event);
            } else {
                grouped.push((aggregate_id, vec![event]));
            }
        }

        let mut streams = Vec::with_capacity(grouped.len());
        for (aggregate_id, events) in grouped {
            let (_, expected_version) = store
                .load_events(aggregate_type, &aggregate_id.to_string())
                .await?;
            streams.push(WorkflowStreamAppend {
                aggregate_id: aggregate_id.to_string(),
                expected_version,
                events,
            });
        }
        Ok(streams)
    }

    /// Appends multi-stream village workflow facts atomically, then projects them.
    ///
    /// Contract:
    /// - all stream writes succeed or none are committed
    /// - stream conflicts fail fast with `CqrsError::Conflict`
    /// - projector dispatch runs only after a successful append
    async fn append_village_workflow_events(
        &self,
        workflow_events: Vec<(u32, parabellum_app::villages::VillageEvent)>,
    ) -> Result<(), CqrsError> {
        if workflow_events.is_empty() {
            return Ok(());
        }

        let aggregate_type = std::any::type_name::<parabellum_app::villages::VillageAggregate>();
        let store = PostgresEventStore::new(crate::EventStoreDb::new(self.pool.clone()));
        let streams = self.build_village_workflow_appends(workflow_events).await?;

        let mut tx = self.pool.begin().await.map_err(CqrsError::domain_source)?;
        let mut stored = store
            .append_workflow_events_in_tx(&mut tx, aggregate_type, &streams)
            .await?;
        stored.sort_by_key(|event| event.global_sequence.unwrap_or(i64::MAX));

        let village_projector = VillageProjector::new(self.pool.clone());
        let report_projector = ReportProjector::new(self.pool.clone());
        for event in &stored {
            village_projector.process_in_tx(&mut tx, event).await?;
            report_projector.process_in_tx(&mut tx, event).await?;
        }
        tx.commit().await.map_err(CqrsError::domain_source)?;
        self.refresh_workflow_snapshots(&streams).await?;
        Ok(())
    }

    async fn refresh_workflow_snapshots(
        &self,
        streams: &[WorkflowStreamAppend],
    ) -> Result<(), CqrsError> {
        let aggregate_type = std::any::type_name::<parabellum_app::villages::VillageAggregate>();
        let event_store = PostgresEventStore::new(crate::EventStoreDb::new(self.pool.clone()));
        let snapshot_store =
            PostgresSnapshotStore::new(crate::EventStoreDb::new(self.pool.clone()));

        for stream in streams {
            let (events, version) = event_store
                .load_events(aggregate_type, &stream.aggregate_id)
                .await?;
            let mut aggregate = parabellum_app::villages::VillageAggregate::default();
            aggregate.set_aggregate_id(
                stream
                    .aggregate_id
                    .parse()
                    .map_err(|_| CqrsError::EventStore("invalid village aggregate id".into()))?,
            );
            aggregate.apply_events(&events).await?;
            aggregate.set_version(version);
            snapshot_store
                .save_snapshot(AggregateSnapshot::new(&aggregate, Some(version))?)
                .await?;
        }
        Ok(())
    }

    pub(super) async fn append_workflow_events(
        &self,
        workflow_events: workflows::WorkflowEvents,
    ) -> Result<(), CqrsError> {
        if workflow_events.is_empty() {
            return Ok(());
        }

        self.append_village_workflow_events(workflow_events.into_inner())
            .await
    }
}
