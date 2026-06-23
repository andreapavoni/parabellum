//! Identifier source used by application use cases.
//!
//! Use cases create canonical command ids through this port so tests can assert
//! behavior with deterministic identifiers.

use uuid::Uuid;

/// Provides generated identifiers to application use cases.
pub trait IdGenerator: Send + Sync {
    /// Returns the next unique identifier for command/workflow intent.
    fn next(&self) -> Uuid;
}

/// Production identifier generator backed by random UUID v4 values.
#[derive(Debug, Default)]
pub struct UuidGenerator;

impl IdGenerator for UuidGenerator {
    fn next(&self) -> Uuid {
        Uuid::new_v4()
    }
}
