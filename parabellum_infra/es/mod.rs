//! Event-sourcing infrastructure for village CQRS runtime.
//!
//! This module owns:
//! - runtime and store wiring
//! - Postgres-backed read-model repositories
//! - event consumers/projectors
//! - scheduled-action worker and replay tooling

mod consumers;
pub(crate) mod lock_keys;
mod replay;
mod repositories;
mod runtime;
mod village_service;
mod stores;
mod stream;
#[cfg(test)]
mod tests;
mod worker;

pub use mini_cqrs_es::{
    Aggregate, AggregateManager, AggregateSnapshot, Command, Cqrs, CqrsError, EventConsumer,
    EventConsumers, EventMetadata, EventPayload, EventStore, NewEvent, Query, QueryRunner,
    Repository, SimpleAggregateManager, SimpleCqrs, SnapshotAggregateManager, SnapshotStore,
    StoredEvent,
};

pub use consumers::{ReportProjector, VillageProjector};
pub use replay::{ReplayMode, ReplayRequest, ReplayService, ReplaySummary, ReplayTarget};
pub use repositories::{
    PostgresArmyRepository, PostgresHeroRepository, PostgresMarketplaceRepository,
    PostgresReportRepository, PostgresScheduledActionRepository, PostgresVillageMovementRepository,
    PostgresVillageRepository,
};
pub use runtime::{VillageCqrsRuntime, village_cqrs_runtime};
pub use village_service::VillageEsService;
pub use stores::{PostgresEventStore, PostgresSnapshotStore};
pub use stream::{VILLAGE_STREAM_TYPE, village_stream_id};
pub use worker::EsScheduledActionWorker;
