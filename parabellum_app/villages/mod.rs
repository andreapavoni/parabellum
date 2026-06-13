mod aggregate;
pub mod commands;
mod events;
mod mapping;
pub mod models;
mod policies;
pub mod queries;
pub mod repositories;
pub mod service;
mod state;

pub use aggregate::VillageAggregate;
pub use commands::{
    AcceptMarketplaceOffer, AddBuilding, ApplyBattleOutcomeToVillage, AttackVillage, BuildTraps,
    CancelBuildingConstruction, CancelMarketplaceOffer, CancelTroopMovement, CompleteTrapBuild,
    CreateHero, CreateMarketplaceOffer, DisbandTrappedTroops, DowngradeBuilding, FoundVillage,
    MarkReportRead, RecallReinforcements, ReleaseReinforcements, ReleaseTrappedTroops,
    RenameVillage, ResearchAcademy, ResearchSmithy, ResolveAttackBattle, ResolveScoutBattle,
    ReviveHero, ScoutVillage, SendMerchantsTransfer, SendReinforcement, SendSettlers,
    SetVillageResources, TrainUnits, UpgradeBuilding,
};
pub use events::VillageEvent;
pub use mapping::{VillageArmyContext, hydrate_village};
pub use policies::army_dispatch::{ArmyDispatch, ArmyDispatchRequest};
pub use policies::expansion::{ConquestAttempt, ExpansionSlotUsage, ExpansionTrainingCommitment};
pub use policies::marketplace::{MarketplaceAcceptance, MarketplaceOfferCreation};
pub use policies::reinforcement_control::ReinforcementControl;
pub use service::VillageService;
pub use state::VillageState;
