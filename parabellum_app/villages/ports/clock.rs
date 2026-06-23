//! Time source used by application use cases.
//!
//! Use cases depend on this port whenever "now" participates in command
//! planning. Production uses `SystemClock`; tests should use a fixed clock.

use chrono::{DateTime, Utc};

/// Provides the current application time to use cases.
pub trait Clock: Send + Sync {
    /// Returns the current UTC timestamp for command planning.
    fn now(&self) -> DateTime<Utc>;
}

/// Production clock backed by `Utc::now`.
#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
