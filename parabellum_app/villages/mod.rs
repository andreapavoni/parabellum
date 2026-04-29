mod aggregate;
mod commands;
mod events;
mod state;
pub mod models;
pub mod repositories;
pub mod service;

pub use aggregate::VillageAggregate;
pub use commands::{
    AddBuilding, CompleteAddBuilding, CompleteDowngradeBuilding, CompleteUpgradeBuilding,
    DowngradeBuilding, FoundVillage, ReinforcementArrived, SendReinforcement, UpgradeBuilding,
};
pub use events::VillageEvent;
pub use state::VillageState;
pub use service::VillageService;
