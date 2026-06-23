//! Identity application contracts.
//!
//! Identity owns authentication, user/player lookup, and player registration
//! orchestration. Infrastructure implements lower-level identity persistence,
//! map reservation, and initial village command execution ports.

pub mod ports;
pub mod requests;
pub mod use_cases;

pub use ports::{
    CreatedRegistrationIdentity, IdentityPort, InitialVillageCommandExecutor, PlayerRepository,
    RegistrationIdentityPort, RegistrationIdentityRecord, UserRepository,
};
pub use requests::{InitialVillageSetup, RegisterPlayerRequest};
pub use use_cases::{RegistrationSettings, RegistrationUseCases};
