//! Projection repository contracts for village read models.
//!
//! These traits define the persistence boundary for ES projectors and
//! read-model queries. `parabellum_infra` supplies concrete Postgres
//! implementations; `parabellum_app` owns only the contracts and read-model
//! shapes needed by command/query orchestration.

pub mod armies;
pub mod expansion;
pub mod heroes;
pub mod marketplace;
pub mod movements;
pub mod reports;
pub mod scheduled_actions;
pub mod villages;

pub use armies::{ArmyListFilter, ArmyRepository, ArmyState};
pub use expansion::{ExpansionCultureSnapshot, ExpansionOwnershipSnapshot};
pub use heroes::HeroRepository;
pub use marketplace::{MarketplaceOfferListFilter, MarketplaceRepository};
pub use movements::VillageMovementRepository;
pub use reports::{ProjectedReport, ReportRepository};
pub use scheduled_actions::{
    ScheduledActionListFilter, ScheduledActionRepository, ScheduledActionVillageFilter,
};
pub use villages::VillageRepository;
