mod aggregate;
pub mod commands;
mod events;
mod mapping;
pub mod models;
pub mod queries;
pub mod repositories;
pub mod service;
mod state;

pub use aggregate::VillageAggregate;
pub use commands::{
    AcceptMarketplaceOffer, AddBuilding, AttackVillage, CancelMarketplaceOffer,
    CompleteAcademyResearch, CompleteAddBuilding, CompleteArmyReturn, CompleteAttackArrival,
    CompleteDowngradeBuilding, CompleteMerchantsArrival, CompleteMerchantsReturn,
    CompleteScoutArrival, CompleteSettlersArrival, CompleteSmithyResearch, CompleteTrainUnit,
    CompleteUpgradeBuilding, ConquerVillage, CreateMarketplaceOffer, DowngradeBuilding,
    FoundVillage, RecallReinforcements, ReinforcementArrived, ReleaseReinforcements,
    ResearchAcademy, ResearchSmithy, ScoutVillage, SendMerchantsTransfer, SendReinforcement,
    SendSettlers, SetVillageResources, TrainUnits, UpgradeBuilding,
};
pub use events::VillageEvent;
pub use service::VillageService;
pub use state::VillageState;
