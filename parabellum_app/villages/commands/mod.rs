//! Village aggregate command set for the CQRS/ES path.
//!
//! Scheduling commands validate preconditions and emit scheduling events.
//! Completion commands apply already-validated work deterministically.
mod accept_marketplace_offer;
mod add_building;
mod apply_battle_outcome_to_village;
mod attack_village;
mod build_traps;
mod cancel_building_construction;
mod cancel_marketplace_offer;
mod cancel_troop_movement;
mod create_hero;
mod create_marketplace_offer;
mod disband_trapped_troops;
mod downgrade_building;
mod found_village;
mod mark_report_read;
mod recall_reinforcements;
mod release_reinforcements;
mod release_trapped_troops;
mod rename_village;
mod research_academy;
mod research_smithy;
mod resolve_attack_battle;
mod resolve_scout_battle;
mod revive_hero;
mod scout_village;
mod send_reinforcement;
mod send_resources;
mod send_settlers;
mod set_village_resources;
mod train_units;
mod upgrade_building;

use mini_cqrs_es::CqrsError;
use std::error::Error;

pub(super) fn as_domain_error<E>(err: E) -> CqrsError
where
    E: Error + Send + Sync + 'static,
{
    CqrsError::domain_source(err)
}

pub(super) fn as_invariant_error<E>(err: E) -> CqrsError
where
    E: Error + Send + Sync + 'static,
{
    CqrsError::invariant_source(err)
}

pub use accept_marketplace_offer::AcceptMarketplaceOffer;
pub use add_building::AddBuilding;
pub use apply_battle_outcome_to_village::ApplyBattleOutcomeToVillage;
pub use attack_village::AttackVillage;
pub use build_traps::{BuildTraps, CompleteTrapBuild};
pub use cancel_building_construction::CancelBuildingConstruction;
pub use cancel_marketplace_offer::CancelMarketplaceOffer;
pub use cancel_troop_movement::CancelTroopMovement;
pub use create_hero::CreateHero;
pub use create_marketplace_offer::CreateMarketplaceOffer;
pub use disband_trapped_troops::DisbandTrappedTroops;
pub use downgrade_building::DowngradeBuilding;
pub use found_village::FoundVillage;
pub use mark_report_read::MarkReportRead;
pub use recall_reinforcements::RecallReinforcements;
pub use release_reinforcements::ReleaseReinforcements;
pub use release_trapped_troops::ReleaseTrappedTroops;
pub use rename_village::RenameVillage;
pub use research_academy::ResearchAcademy;
pub use research_smithy::ResearchSmithy;
pub use resolve_attack_battle::ResolveAttackBattle;
pub use resolve_scout_battle::ResolveScoutBattle;
pub use revive_hero::ReviveHero;
pub use scout_village::ScoutVillage;
pub use send_reinforcement::SendReinforcement;
pub use send_resources::SendMerchantsTransfer;
pub use send_settlers::SendSettlers;
pub use set_village_resources::SetVillageResources;
pub use train_units::TrainUnits;
pub use upgrade_building::UpgradeBuilding;
