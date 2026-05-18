//! Village aggregate command set for the CQRS/ES path.
//!
//! Scheduling commands validate preconditions and emit scheduling events.
//! Completion commands apply already-validated work deterministically.
mod accept_marketplace_offer;
mod add_building;
mod apply_battle_outcome_to_village;
mod attack_village;
mod cancel_marketplace_offer;
mod complete_academy_research;
mod complete_add_building;
mod complete_army_return;
mod complete_attack_arrival;
mod complete_downgrade_building;
mod complete_hero_revival;
mod complete_merchant_return;
mod complete_scout_arrival;
mod complete_settlers_arrival;
mod complete_smithy_research;
mod complete_train_unit;
mod complete_upgrade_building;
mod create_hero;
mod create_marketplace_offer;
mod downgrade_building;
mod found_village;
mod recall_reinforcements;
mod resolve_attack_battle;
mod reinforcement_arrived;
mod release_reinforcements;
mod research_academy;
mod research_smithy;
mod revive_hero;
mod scout_village;
mod send_reinforcement;
mod send_resources;
mod send_settlers;
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
pub use apply_battle_outcome_to_village::ApplyBattleOutcomeToVillage;
pub use attack_village::AttackVillage;
pub use cancel_marketplace_offer::CancelMarketplaceOffer;
pub use complete_academy_research::CompleteAcademyResearch;
pub use complete_add_building::CompleteAddBuilding;
pub use complete_army_return::CompleteArmyReturn;
pub use complete_attack_arrival::CompleteAttackArrival;
pub use complete_downgrade_building::CompleteDowngradeBuilding;
pub use complete_hero_revival::CompleteHeroRevival;
pub use complete_merchant_return::CompleteMerchantsReturn;
pub use complete_scout_arrival::CompleteScoutArrival;
pub use complete_settlers_arrival::CompleteSettlersArrival;
pub use complete_smithy_research::CompleteSmithyResearch;
pub use complete_train_unit::CompleteTrainUnit;
pub use complete_upgrade_building::CompleteUpgradeBuilding;
pub use create_hero::CreateHero;
pub use create_marketplace_offer::CreateMarketplaceOffer;
pub use downgrade_building::DowngradeBuilding;
pub use found_village::FoundVillage;
pub use recall_reinforcements::RecallReinforcements;
pub use resolve_attack_battle::ResolveAttackBattle;
pub use reinforcement_arrived::ReinforcementArrived;
pub use release_reinforcements::ReleaseReinforcements;
pub use research_academy::ResearchAcademy;
pub use research_smithy::ResearchSmithy;
pub use revive_hero::ReviveHero;
pub use scout_village::ScoutVillage;
pub use send_reinforcement::SendReinforcement;
pub use send_resources::SendMerchantsTransfer;
pub use send_settlers::SendSettlers;
pub use set_village_resources::SetVillageResources;
pub use train_units::TrainUnits;
pub use upgrade_building::UpgradeBuilding;
