mod consumers;
mod repositories;
mod runtime;
mod service;
mod stores;
mod stream;
#[cfg(test)]
mod tests;

pub use mini_cqrs_es::{
    Aggregate, AggregateManager, AggregateSnapshot, Command, Cqrs, CqrsError, EventConsumer,
    EventConsumers, EventMetadata, EventPayload, EventStore, NewEvent, Query, QueryRunner,
    Repository, SimpleAggregateManager, SimpleCqrs, SnapshotAggregateManager, SnapshotStore,
    StoredEvent,
};

pub use consumers::VillageProjector;
pub use repositories::{
    PostgresScheduledActionRepository, PostgresVillageModelRepository,
    PostgresVillageMovementRepository,
};
pub use runtime::{VillageCqrsRuntime, village_cqrs_runtime};
pub use service::VillageEsService;
pub use stores::{PostgresEventStore, PostgresSnapshotStore};
pub use stream::{VILLAGE_STREAM_TYPE, village_stream_id};
