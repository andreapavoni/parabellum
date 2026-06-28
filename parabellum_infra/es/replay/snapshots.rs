//! Aggregate snapshot rebuild support.

use mini_cqrs_es::{Aggregate, AggregateSnapshot, CqrsError, EventStore, SnapshotStore};
use parabellum_app::villages::VillageAggregate;
use tracing::info;

use crate::es::{PostgresEventStore, PostgresSnapshotStore};

use super::ReplayService;

impl ReplayService {
    /// Rebuilds aggregate snapshots for all village event streams.
    ///
    /// Queries every distinct `VillageAggregate` ID from `es_events`, replays all
    /// events for each through `VillageAggregate::apply_events`, and writes the
    /// resulting state to `es_snapshots`.
    pub async fn rebuild_all_snapshots(&self) -> Result<i64, CqrsError> {
        let aggregate_type = std::any::type_name::<VillageAggregate>();
        let ids: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT aggregate_id FROM es_events WHERE aggregate_type = $1 ORDER BY aggregate_id",
        )
        .bind(aggregate_type)
        .fetch_all(&self.pool)
        .await
        .map_err(CqrsError::domain_source)?;

        let event_store = PostgresEventStore::new(crate::EventStoreDb::new(self.pool.clone()));
        let snapshot_store =
            PostgresSnapshotStore::new(crate::EventStoreDb::new(self.pool.clone()));
        let mut count = 0i64;

        for (aggregate_id,) in &ids {
            let (events, version) = event_store.load_events(aggregate_type, aggregate_id).await?;

            let mut aggregate = VillageAggregate::default();
            aggregate.set_aggregate_id(aggregate_id.parse::<u32>().map_err(|_| {
                CqrsError::EventStore(format!("invalid aggregate id: {aggregate_id}"))
            })?);
            aggregate.apply_events(&events).await?;
            aggregate.set_version(version);

            snapshot_store
                .save_snapshot(AggregateSnapshot::new(&aggregate, Some(version))?)
                .await?;

            count += 1;
        }

        info!(count, "snapshots rebuilt");
        Ok(count)
    }
}
