//! Postgres-backed event and snapshot stores.
//!
//! The event store persists append-only domain facts in `es_events`. Snapshot
//! storage is kept separate because snapshots are derived operational state and
//! can be rebuilt from events.

mod event_store;
mod rows;
mod snapshots;

pub use event_store::{PostgresEventStore, WorkflowStreamAppend};
pub use snapshots::PostgresSnapshotStore;
