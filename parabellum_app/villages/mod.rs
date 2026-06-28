mod aggregate;
pub mod commands;
mod cqrs_command_service;
pub mod cqrs_queries;
mod events;
mod mapping;
pub mod models;
mod policies;
pub mod ports;
pub mod projection_repositories;
pub mod read_models;
pub mod requests;
mod state;
pub mod use_cases;

pub use aggregate::VillageAggregate;
pub use commands::{
    AcceptMarketplaceOffer, AddBuilding, ApplyBattleOutcomeToVillage, AssignHeroPoints,
    AttackVillage, BuildTraps, CancelBuildingConstruction, CancelMarketplaceOffer,
    CancelTroopMovement, CompleteTrapBuild, CreateHero, CreateMarketplaceOffer,
    DisbandTrappedTroops, DowngradeBuilding, FoundVillage, MarkReportRead, RecallReinforcements,
    ReleaseReinforcements, ReleaseTrappedTroops, RenameVillage, ResearchAcademy, ResearchSmithy,
    ResetHeroPoints, ResolveAttackBattle, ResolveScoutBattle, ReviveHero, ScoutVillage,
    SendMerchantsTransfer, SendReinforcement, SendSettlers, SetHeroResourceFocus,
    SetVillageResources, TrainUnits, UpgradeBuilding,
};
pub use cqrs_command_service::VillageService;
pub use events::VillageEvent;
pub use mapping::{VillageArmyContext, apply_domain_village_state, hydrate_village};
pub use policies::army_dispatch::{ArmyDispatch, ArmyDispatchRequest};
pub use policies::expansion::{ConquestAttempt, ExpansionSlotUsage, ExpansionTrainingCommitment};
pub use policies::marketplace::{MarketplaceAcceptance, MarketplaceOfferCreation};
pub use policies::reinforcement_control::ReinforcementControl;
pub use ports::{
    BuildingCommandExecutor, BuildingCommandIntent, BuildingReadPort,
    CancelBuildingConstructionContext, CancelTroopMovementContext, Clock,
    DevelopmentCommandExecutor, DevelopmentCommandIntent, DevelopmentReadPort, ExpansionReadPort,
    HeroCommandExecutor, HeroCommandIntent, HeroReadPort, IdGenerator, MarketplaceCommandExecutor,
    MarketplaceCommandIntent, MarketplaceReadPort, MovementControlCommandExecutor,
    MovementControlCommandIntent, MovementControlReadPort, MovementReadPort,
    ReinforcementArmyContext, ReinforcementCommandExecutor, ReinforcementCommandIntent,
    ReinforcementReadPort, ReportCommandExecutor, ReportCommandIntent, ReportReadPort, SystemClock,
    TrapCommandExecutor, TrapCommandIntent, TrapReadPort, TrappedArmyContext, UuidGenerator,
    VillageActivityReadPort, VillageArmyReadPort, VillageCommandExecutor, VillageCommandIntent,
    VillageProfileCommandExecutor, VillageProfileCommandIntent, VillageReferenceReadPort,
    VillageStateReadPort,
};
pub use read_models::{
    AcademyQueueItem, BuildingQueueItem, MarketplaceData, MerchantMovement,
    MerchantMovementDirection, MerchantMovementKind, SmithyQueueItem, TrainingQueueItem,
    TrapQueueItem, TroopMovement, TroopMovementDirection, TroopMovementType, VillageArmyStateView,
    VillageQueues, VillageTroopMovements,
};
pub use requests::activity::{
    GetVillageQueuesRequest, GetVillageTroopMovementsRequest,
    ListCancelableOutgoingMovementIdsRequest,
};
pub use requests::buildings::{
    AddBuildingRequest, CancelBuildingConstructionRequest, DowngradeBuildingRequest,
    UpgradeBuildingRequest,
};
pub use requests::development::{ResearchAcademyRequest, ResearchSmithyRequest, TrainUnitsRequest};
pub use requests::expansion::GetExpansionCultureInfoRequest;
pub use requests::heroes::{
    AssignHeroPointsRequest, CreateHeroRequest, GetHeroByPlayerRequest,
    GetPendingHeroRevivalRequest, ResetHeroPointsRequest, ReviveHeroRequest,
    SetHeroResourceFocusRequest,
};
pub use requests::marketplace::{
    AcceptMarketplaceOfferRequest, CancelMarketplaceOfferRequest, CreateMarketplaceOfferRequest,
    GetMarketplaceDataRequest, GetMarketplaceOfferRequest, SendResourcesRequest,
};
pub use requests::movement_control::CancelTroopMovementRequest;
pub use requests::movements::{
    SendAttackRequest, SendReinforcementRequest, SendScoutRequest, SendSettlersRequest,
};
pub use requests::reinforcements::{
    DisbandTrappedTroopsRequest, RecallReinforcementsRequest, ReleaseReinforcementsRequest,
    ReleaseTrappedTroopsRequest,
};
pub use requests::reports::{
    CountUnreadReportsForPlayerRequest, GetReportForPlayerRequest, ListReportsForPlayerRequest,
    MarkReportReadRequest,
};
pub use requests::traps::BuildTrapsRequest;
pub use requests::village_army::GetVillageArmyStateViewRequest;
pub use requests::village_profile::RenameVillageRequest;
pub use requests::village_references::GetVillageReferencesRequest;
pub use requests::village_state::{GetVillageStateRequest, ListPlayerVillageStatesRequest};
pub use state::VillageState;
pub use use_cases::{
    BuildingSettings, BuildingUseCases, DevelopmentSettings, DevelopmentUseCases,
    ExpansionCultureInfo, HeroSettings, HeroUseCases, MarketplaceSettings, MarketplaceUseCases,
    MovementControlUseCases, MovementSettings, MovementUseCases, ReinforcementSettings,
    ReinforcementUseCases, ReportUseCases, TrapUseCases, VillageActivityUseCases,
    VillageArmyUseCases, VillageExpansionUseCases, VillageProfileUseCases,
    VillageReferenceUseCases, VillageStateUseCases,
};
