//! Replay tooling for rebuilding projection read models from canonical events.

mod filters;
mod request;
mod runner;
mod snapshots;

use sqlx::PgPool;

use crate::es::PostgresEventStore;

pub use request::{ReplayMode, ReplayRequest, ReplaySummary, ReplayTarget};

#[derive(Debug, Clone)]
pub struct ReplayService {
    pub(super) pool: PgPool,
    pub(super) event_store: PostgresEventStore,
}

impl ReplayService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            event_store: PostgresEventStore::new(crate::EventStoreDb::new(pool.clone())),
            pool,
        }
    }
}
