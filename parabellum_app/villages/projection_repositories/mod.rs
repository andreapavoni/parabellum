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
pub mod merchant_movements;
pub mod movements;
pub mod reports;
pub mod scheduled_actions;
pub mod villages;

pub use armies::{ArmyListFilter, ArmyRepository, ArmyState};
pub use expansion::{ExpansionCultureSnapshot, ExpansionOwnershipSnapshot};
pub use heroes::{HeroPlacementState, HeroRepository};
pub use marketplace::{MarketplaceOfferListFilter, MarketplaceRepository};
pub use merchant_movements::MerchantMovementRepository;
pub use movements::{VillageMovementFilter, VillageMovementRepository};
pub use reports::{ProjectedReport, ReportFilter, ReportKind, ReportRepository};
pub use scheduled_actions::{
    ScheduledActionFilter, ScheduledActionOrder, ScheduledActionRepository,
    ScheduledActionWorkflowFilter,
};
pub use villages::VillageRepository;
