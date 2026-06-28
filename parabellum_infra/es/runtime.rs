use mini_cqrs_es::{EventConsumers, SimpleCqrs, SnapshotAggregateManager};
use sqlx::PgPool;

use crate::es::{PostgresEventStore, PostgresSnapshotStore, ReportProjector, VillageProjector};

pub type VillageCqrsRuntime =
    SimpleCqrs<PostgresEventStore, SnapshotAggregateManager<PostgresSnapshotStore>>;

pub fn village_cqrs_runtime(pool: PgPool) -> VillageCqrsRuntime {
    let event_store = PostgresEventStore::new(crate::EventStoreDb::new(pool.clone()));
    let aggregate_manager = SnapshotAggregateManager::new(PostgresSnapshotStore::new(
        crate::EventStoreDb::new(pool.clone()),
    ));
    let consumers = EventConsumers::new()
        .with(ReportProjector::new(pool.clone()))
        .with(VillageProjector::new(pool));

    SimpleCqrs::new(aggregate_manager, event_store, consumers)
}
