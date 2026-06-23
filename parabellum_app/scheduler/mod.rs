//! Scheduler application contracts.
//!
//! Scheduler use cases trigger due scheduled workflow execution through an
//! infrastructure port. The actual workflow processing and CQRS/ES transaction
//! handling remain infrastructure concerns.

pub mod ports;
pub mod requests;
pub mod use_cases;

pub use ports::SchedulerPort;
pub use requests::ProcessDueActionsRequest;
pub use use_cases::SchedulerUseCases;
