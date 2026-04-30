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
    AcceptMarketplaceOffer, AddBuilding, CancelMarketplaceOffer, CompleteAcademyResearch,
    CompleteAddBuilding, CompleteDowngradeBuilding, CompleteMerchantsArrival,
    CompleteMerchantsReturn, CompleteSmithyResearch, CompleteTrainUnit, CompleteUpgradeBuilding,
    CreateMarketplaceOffer, DowngradeBuilding, FoundVillage, ReinforcementArrived, ResearchAcademy,
    ResearchSmithy, SendMerchantsTransfer, SendReinforcement, SetVillageResources, TrainUnits,
    UpgradeBuilding,
};
pub use events::VillageEvent;
pub use service::VillageService;
pub use state::VillageState;
