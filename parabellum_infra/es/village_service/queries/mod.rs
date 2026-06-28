//! Query/read helpers for `VillageEsService`.
//!
//! These modules are side-effect free with respect to aggregate mutations,
//! except for command-backed report state changes documented in `reports`.

mod buildings;
mod heroes;
mod marketplace;
mod movements;
mod reports;
mod scheduled_actions;
mod villages;
