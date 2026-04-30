use mini_cqrs_es::{EventConsumers, SimpleAggregateManager, SimpleCqrs};
use sqlx::PgPool;

use crate::es::{PostgresEventStore, VillageProjector};

pub type VillageCqrsRuntime =
    SimpleCqrs<PostgresEventStore, SimpleAggregateManager<PostgresEventStore>>;

pub fn village_cqrs_runtime(pool: PgPool) -> VillageCqrsRuntime {
    let event_store = PostgresEventStore::new(pool.clone());
    let aggregate_manager = SimpleAggregateManager::new(event_store.clone());
    let consumers = EventConsumers::new().with(VillageProjector::new(pool));

    SimpleCqrs::new(aggregate_manager, event_store, consumers)
}
