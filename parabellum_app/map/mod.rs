//! Map application concern.
//!
//! Map reads are cross-context application queries. They use a focused
//! `MapReadPort` and stay outside village query/read-model ports.

pub mod ports;
pub mod requests;
pub mod use_cases;

pub use ports::MapReadPort;
pub use requests::{GetMapFieldRequest, GetMapRegionRequest, GetMapRegionTileByFieldIdRequest};
pub use use_cases::MapUseCases;
