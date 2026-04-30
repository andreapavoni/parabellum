mod aggregate;
pub mod commands;
mod events;
pub mod models;
pub mod queries;
pub mod repositories;
pub mod service;
mod state;

pub use aggregate::VillageAggregate;
pub use commands::{
    AddBuilding, CompleteAcademyResearch, CompleteAddBuilding, CompleteDowngradeBuilding,
    CompleteSmithyResearch, CompleteTrainUnit, CompleteUpgradeBuilding, DowngradeBuilding,
    FoundVillage, ReinforcementArrived, ResearchAcademy, ResearchSmithy, SendReinforcement,
    SetVillageResources, TrainUnits, UpgradeBuilding,
};
pub use events::VillageEvent;
pub use service::VillageService;
pub use state::VillageState;
