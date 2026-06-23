//! Village CQRS and projection model contracts.
//!
//! These types describe app-owned projection rows and scheduled workflow
//! payloads used by the CQRS/ES runtime. They are split by concern while this
//! module keeps `villages::models::...` as the public import path.

pub mod marketplace;
pub mod movements;
pub mod reports;
pub mod scheduled_actions;
pub mod villages;
pub mod workflows;

pub use marketplace::{MarketplaceOfferModel, MarketplaceOfferSnapshot, MarketplaceOfferStatus};
pub use movements::{MovementDirection, MovementType, VillageMovement, VillageTroopMovements};
pub use reports::ReportModel;
pub use scheduled_actions::{
    ScheduledAction, ScheduledActionPayload, ScheduledActionStatus, ScheduledActionType,
};
pub use villages::VillageModel;
pub use workflows::{
    ArmyReturnWorkflow, AttackArrivalWorkflow, BuildingWorkflow, BuildingWorkflowKind,
    HeroRevivalWorkflow, MerchantArrivalWorkflow, MerchantReturnWorkflow,
    ReinforcementArrivalWorkflow, ResearchWorkflow, ResearchWorkflowKind, ScoutArrivalWorkflow,
    SettlersArrivalWorkflow, TrainingWorkflow, TrapBuildWorkflow, TrappedTroopReturn,
};
