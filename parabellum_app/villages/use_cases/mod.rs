//! Village application use cases.
//!
//! Use cases orchestrate app context, call `parabellum_game` for pure
//! mechanics, and delegate persistence/execution through app ports.

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

pub use activity::VillageActivityUseCases;
pub use buildings::{BuildingSettings, BuildingUseCases};
pub use development::{DevelopmentSettings, DevelopmentUseCases};
pub use expansion::{ExpansionCultureInfo, VillageExpansionUseCases};
pub use heroes::{HeroSettings, HeroUseCases};
pub use marketplace::{MarketplaceSettings, MarketplaceUseCases};
pub use movement_control::MovementControlUseCases;
pub use movements::{MovementSettings, MovementUseCases};
pub use reinforcements::{ReinforcementSettings, ReinforcementUseCases};
pub use reports::ReportUseCases;
pub use traps::TrapUseCases;
pub use village_army::VillageArmyUseCases;
pub use village_profile::VillageProfileUseCases;
pub use village_references::VillageReferenceUseCases;
pub use village_state::VillageStateUseCases;
