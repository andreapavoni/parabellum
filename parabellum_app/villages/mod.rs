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
    AcceptMarketplaceOffer, AddBuilding, ApplyBattleOutcomeToVillage, AttackVillage,
    CancelMarketplaceOffer, CreateHero, CreateMarketplaceOffer, DowngradeBuilding, FoundVillage,
    RecallReinforcements, ReleaseReinforcements, ResearchAcademy, ResearchSmithy,
    ResolveAttackBattle, ReviveHero, ScoutVillage, SendMerchantsTransfer, SendReinforcement,
    SendSettlers, SetVillageResources, TrainUnits, UpgradeBuilding,
};
pub use events::VillageEvent;
pub use service::VillageService;
pub use state::VillageState;
