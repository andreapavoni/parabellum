//! Village aggregate command set for the CQRS/ES path.
//!
//! Scheduling commands validate preconditions and emit scheduling events.
//! Completion commands apply already-validated work deterministically.
mod accept_marketplace_offer;
mod add_building;
mod cancel_marketplace_offer;
mod complete_academy_research;
mod complete_add_building;
mod complete_downgrade_building;
mod complete_merchant_arrival;
mod complete_merchant_return;
mod complete_smithy_research;
mod complete_train_unit;
mod complete_upgrade_building;
mod create_marketplace_offer;
mod downgrade_building;
mod found_village;
mod reinforcement_arrived;
mod research_academy;
mod research_smithy;
mod send_reinforcement;
mod send_resources;
mod set_village_resources;
mod train_units;
mod upgrade_building;

use mini_cqrs_es::CqrsError;
use std::fmt::Display;

pub(super) fn as_domain_error<E: Display>(err: E) -> CqrsError {
    CqrsError::domain(err)
}

pub(super) fn as_invariant_error<E: Display>(err: E) -> CqrsError {
    CqrsError::invariant(err)
}

pub use accept_marketplace_offer::AcceptMarketplaceOffer;
pub use add_building::AddBuilding;
pub use cancel_marketplace_offer::CancelMarketplaceOffer;
pub use complete_academy_research::CompleteAcademyResearch;
pub use complete_add_building::CompleteAddBuilding;
pub use complete_downgrade_building::CompleteDowngradeBuilding;
pub use complete_merchant_arrival::CompleteMerchantsArrival;
pub use complete_merchant_return::CompleteMerchantsReturn;
pub use complete_smithy_research::CompleteSmithyResearch;
pub use complete_train_unit::CompleteTrainUnit;
pub use complete_upgrade_building::CompleteUpgradeBuilding;
pub use create_marketplace_offer::CreateMarketplaceOffer;
pub use downgrade_building::DowngradeBuilding;
pub use found_village::FoundVillage;
pub use reinforcement_arrived::ReinforcementArrived;
pub use research_academy::ResearchAcademy;
pub use research_smithy::ResearchSmithy;
pub use send_reinforcement::SendReinforcement;
pub use send_resources::SendMerchantsTransfer;
pub use set_village_resources::SetVillageResources;
pub use train_units::TrainUnits;
pub use upgrade_building::UpgradeBuilding;
