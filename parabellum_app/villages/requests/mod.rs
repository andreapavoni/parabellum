//! Transport-independent village use-case inputs.
//!
//! Request types in this module are accepted by application use cases. They are
//! not HTTP DTOs and they are not aggregate commands; use cases translate them
//! into canonical command intent after loading the required app context.

pub mod activity;
pub mod buildings;
pub mod development;
pub mod expansion;
pub mod heroes;
pub mod marketplace;
pub mod movement_control;
pub mod movements;
pub mod reinforcements;
pub mod reports;
pub mod traps;
pub mod village_army;
pub mod village_profile;
pub mod village_references;
pub mod village_state;
