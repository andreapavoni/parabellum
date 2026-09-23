use mini_cqrs_es::{
    Aggregate, AggregateSnapshot, Command, Cqrs, CqrsError, EventMetadata, NewEvent, QueryRunner,
};
use sqlx::PgPool;

use crate::es::{
    PostgresEventStore, PostgresSnapshotStore, ReportProjector, VillageProjector,
    WorkflowStreamAppend,
};

/// Commits command events, projections, and snapshots in one transaction.
pub struct VillageCqrsRuntime {
    pool: PgPool,
}

/// Builds the transactional runtime over the shared PostgreSQL pool.
pub fn village_cqrs_runtime(pool: PgPool) -> VillageCqrsRuntime {
    VillageCqrsRuntime { pool }
}

impl QueryRunner for VillageCqrsRuntime {}

impl Cqrs for VillageCqrsRuntime {
    async fn execute<C: Command>(
        &self,
        aggregate_id: &<C::Aggregate as Aggregate>::Id,
        command: &C,
    ) -> Result<<C::Aggregate as Aggregate>::Id, CqrsError> {
        let mut tx = self.pool.begin().await.map_err(CqrsError::domain_source)?;
        // Replay takes SHARE before resetting projections. Acquire our writer
        // lock before reading state so commands cannot straddle a rebuild.
        sqlx::query("LOCK TABLE es_events IN ROW EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await
            .map_err(CqrsError::domain_source)?;
        let store = PostgresEventStore::new(crate::EventStoreDb::new(self.pool.clone()));
        let mut aggregate = store
            .load_aggregate_in_tx::<C::Aggregate>(&mut tx, aggregate_id)
            .await?;
        let events = command
            .handle(&aggregate)
            .await?
            .into_iter()
            .map(|payload| NewEvent::from_payload(payload, EventMetadata::default()))
            .collect::<Result<Vec<_>, _>>()?;
        let stream = WorkflowStreamAppend {
            aggregate_id: aggregate_id.to_string(),
            expected_version: aggregate.version(),
            events,
        };
        let stored = store
            .append_workflow_events_in_tx(&mut tx, std::any::type_name::<C::Aggregate>(), &[stream])
            .await?;
        let reports = ReportProjector::new(self.pool.clone());
        let villages = VillageProjector::new(self.pool.clone());
        for event in &stored {
            reports.process_in_tx(&mut tx, event).await?;
            villages.process_in_tx(&mut tx, event).await?;
        }
        aggregate.apply_events(&stored).await?;
        if let Some(event) = stored.last() {
            aggregate.set_version(event.version);
        }
        PostgresSnapshotStore::new(crate::EventStoreDb::new(self.pool.clone()))
            .save_in_tx(
                &mut tx,
                AggregateSnapshot::new(&aggregate, Some(aggregate.version()))?,
            )
            .await?;
        tx.commit().await.map_err(CqrsError::domain_source)?;
        Ok(aggregate_id.clone())
    }
}
