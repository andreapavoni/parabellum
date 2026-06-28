//! Logical persistence boundaries used by infrastructure components.
//!
//! Events are canonical history. Projection/read-model rows are rebuildable
//! operational state. The two boundaries share one `PgPool` today, but keeping
//! them distinct in constructors makes future schema/database separation
//! explicit.

use sqlx::PgPool;

/// Database handle for canonical event-store tables.
#[derive(Debug, Clone)]
pub struct EventStoreDb {
    pool: PgPool,
}

impl EventStoreDb {
    /// Creates an event-store boundary from an existing Postgres pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Returns the underlying pool for infra-local SQL execution.
    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl From<PgPool> for EventStoreDb {
    fn from(pool: PgPool) -> Self {
        Self::new(pool)
    }
}

/// Database handle for rebuildable projection and read-model tables.
#[derive(Debug, Clone)]
pub struct ProjectionDb {
    pool: PgPool,
}

impl ProjectionDb {
    /// Creates a projection-store boundary from an existing Postgres pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Returns the underlying pool for infra-local SQL execution.
    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl From<PgPool> for ProjectionDb {
    fn from(pool: PgPool) -> Self {
        Self::new(pool)
    }
}

/// Logical persistence handles for one physical infrastructure database.
#[derive(Debug, Clone)]
pub struct InfraDb {
    pub events: EventStoreDb,
    pub projections: ProjectionDb,
}

impl InfraDb {
    /// Creates both logical boundaries from the same physical Postgres pool.
    pub fn shared(pool: PgPool) -> Self {
        Self {
            events: EventStoreDb::new(pool.clone()),
            projections: ProjectionDb::new(pool),
        }
    }
}
