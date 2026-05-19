//! Village aggregate domain events for CQRS/ES flows.
//!
//! Event families:
//! - immediate actions (e.g. `VillageFounded`, `ReinforcementSent`)
//! - scheduled actions (`*Scheduled`)
//! - deterministic completions (`*Completed`, `Building*`)
//! - utility updates (`VillageResourcesSet`)
use std::fmt;

use chrono::{DateTime, Utc};
use mini_cqrs_es::EventPayload;
use parabellum_game::battle::BattleReport;
use parabellum_game::models::army::Army;
use parabellum_game::models::hero::Hero;
use parabellum_game::models::village::{VillageBuilding, VillageProduction, VillageStocks};
use parabellum_types::army::UnitName;
use parabellum_types::battle::{AttackType, ScoutingTarget};
use parabellum_types::buildings::BuildingName;
use parabellum_types::common::{ResourceGroup, ResourceQuantity};
use parabellum_types::map::Position;
use parabellum_types::tribe::Tribe;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VillageEvent {
    VillageFounded {
        village_id: u32,
        village_name: String,
        position: Position,
        tribe: Tribe,
        player_id: Uuid,
        parent_village_id: Option<u32>,
        buildings: Vec<VillageBuilding>,
    },
    VillageConquered {
        player_id: Uuid,
        owner_village_id: u32,
    },
    /// Emitted when resources are explicitly set through `SetVillageResources`.
    ///
    /// Projectors should update resource-dependent read models directly.
    VillageResourcesSet {
        player_id: Uuid,
        village_id: u32,
        resources: ResourceGroup,
    },
    VillageArmyDetached {
        army: Army,
    },
    HeroCreated {
        player_id: Uuid,
        village_id: u32,
        hero: Hero,
    },
    HeroRevivalScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        hero: Hero,
        reset: bool,
        revive_at: DateTime<Utc>,
        cost: ResourceGroup,
    },
    HeroRevived {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        hero: Hero,
        reset: bool,
        revived_at: DateTime<Utc>,
    },
    ReinforcementSent {
        movement_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        army: Army,
        arrives_at: DateTime<Utc>,
    },
    ReinforcementArrived {
        movement_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        army: Army,
        hero_alone_transfer: bool,
        arrives_at: DateTime<Utc>,
    },
    ReinforcementAppliedToVillage {
        movement_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        army: Army,
        hero_alone_transfer: bool,
        arrives_at: DateTime<Utc>,
    },
    ReinforcementsRecalled {
        action_id: Uuid,
        movement_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        home_village_id: u32,
        stationed_village_id: u32,
        army: Army,
        returns_at: DateTime<Utc>,
    },
    ReinforcementsReleased {
        action_id: Uuid,
        movement_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        home_village_id: u32,
        stationed_village_id: u32,
        army: Army,
        returns_at: DateTime<Utc>,
    },
    SettlersSent {
        action_id: Uuid,
        movement_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        target_position: Position,
        village_name: String,
        tribe: Tribe,
        army: Army,
        arrives_at: DateTime<Utc>,
    },
    SettlersArrived {
        action_id: Uuid,
        movement_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        target_position: Position,
        village_name: String,
        tribe: Tribe,
        arrives_at: DateTime<Utc>,
    },
    AttackSent {
        movement_id: Uuid,
        army_id: Uuid,
        arrival_action_id: Uuid,
        return_action_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        army: Army,
        attack_type: AttackType,
        catapult_targets: [BuildingName; 2],
        arrives_at: DateTime<Utc>,
        returns_at: DateTime<Utc>,
    },
    AttackArrivalScheduled {
        action_id: Uuid,
        movement_id: Uuid,
        return_action_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        army_id: Uuid,
        army: Army,
        attack_type: AttackType,
        catapult_targets: [BuildingName; 2],
        arrives_at: DateTime<Utc>,
        returns_at: DateTime<Utc>,
    },
    AttackArrived {
        movement_id: Uuid,
        army_id: Uuid,
        action_id: Uuid,
        return_action_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        army: Army,
        attack_type: AttackType,
        catapult_targets: [BuildingName; 2],
        arrives_at: DateTime<Utc>,
        returns_at: DateTime<Utc>,
    },
    AttackBattleResolved {
        action_id: Uuid,
        movement_id: Uuid,
        return_action_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        attack_type: AttackType,
        report: BattleReport,
        returning_army: Option<Army>,
        stationed_attacker_army: Option<Army>,
        returns_at: DateTime<Utc>,
    },
    /// Canonical target-stream fact for battle side effects.
    ///
    /// This payload is the authoritative post-battle state projection input for
    /// the target village at the time of append.
    BattleOutcomeAppliedToVillage {
        action_id: Uuid,
        movement_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        target_player_id: Uuid,
        target_parent_village_id: Option<u32>,
        target_loyalty: u8,
        target_buildings: Vec<VillageBuilding>,
        target_production: VillageProduction,
        target_population: u32,
        target_stocks: VillageStocks,
        target_army: Option<Army>,
        target_reinforcements: Vec<Army>,
        stationed_attacker_army: Option<Army>,
    },
    ArmyReturned {
        action_id: Uuid,
        movement_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        army: Army,
        bounty: Option<ResourceGroup>,
        returns_at: DateTime<Utc>,
    },
    ScoutSent {
        movement_id: Uuid,
        army_id: Uuid,
        arrival_action_id: Uuid,
        return_action_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        army: Army,
        target: ScoutingTarget,
        attack_type: AttackType,
        arrives_at: DateTime<Utc>,
        returns_at: DateTime<Utc>,
    },
    ScoutArrived {
        movement_id: Uuid,
        army_id: Uuid,
        action_id: Uuid,
        return_action_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        army: Army,
        target: ScoutingTarget,
        attack_type: AttackType,
        arrives_at: DateTime<Utc>,
        returns_at: DateTime<Utc>,
    },
    /// Canonical source-stream fact for scout battle resolution.
    ///
    /// This carries the resolved scout battle report and the optional returning
    /// army snapshot used to project return movement/scheduling.
    ScoutBattleResolved {
        action_id: Uuid,
        movement_id: Uuid,
        return_action_id: Uuid,
        army_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        attack_type: AttackType,
        report: BattleReport,
        returning_army: Option<Army>,
        returns_at: DateTime<Utc>,
    },
    MerchantsTripScheduled {
        arrival_action_id: Uuid,
        return_action_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        resources: ResourceGroup,
        merchants_used: u8,
        resources_already_reserved: bool,
        arrives_at: DateTime<Utc>,
        returns_at: DateTime<Utc>,
    },
    MerchantsArrived {
        action_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        resources: ResourceGroup,
        merchants_used: u8,
        arrives_at: DateTime<Utc>,
    },
    /// Canonical target-stream fact for merchant transfer arrival materialization.
    ///
    /// `target_stocks` is the persisted stock snapshot computed by the workflow
    /// at append time.
    MerchantTransferAppliedToVillage {
        action_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        resources: ResourceGroup,
        merchants_used: u8,
        arrives_at: DateTime<Utc>,
        target_stocks: VillageStocks,
    },
    MerchantsReturned {
        action_id: Uuid,
        player_id: Uuid,
        source_village_id: u32,
        merchants_used: u8,
        returns_at: DateTime<Utc>,
    },
    MarketplaceOfferCreated {
        offer_id: Uuid,
        owner_player_id: Uuid,
        owner_village_id: u32,
        offer_resources: ResourceQuantity,
        seek_resources: ResourceQuantity,
        merchants_reserved: u8,
        created_at: DateTime<Utc>,
    },
    MarketplaceOfferReservationAppliedToVillage {
        offer_id: Uuid,
        owner_player_id: Uuid,
        owner_village_id: u32,
        merchants_reserved: u8,
        owner_stocks: VillageStocks,
        owner_busy_merchants: u8,
        applied_at: DateTime<Utc>,
    },
    MarketplaceOfferCanceled {
        offer_id: Uuid,
        owner_player_id: Uuid,
        owner_village_id: u32,
        offer_resources: ResourceQuantity,
        merchants_reserved: u8,
        canceled_at: DateTime<Utc>,
    },
    MarketplaceOfferReservationReleasedFromVillage {
        offer_id: Uuid,
        owner_player_id: Uuid,
        owner_village_id: u32,
        merchants_reserved: u8,
        owner_stocks: VillageStocks,
        owner_busy_merchants: u8,
        released_at: DateTime<Utc>,
    },
    MarketplaceOfferAccepted {
        offer_id: Uuid,
        owner_player_id: Uuid,
        owner_village_id: u32,
        accepting_player_id: Uuid,
        accepting_village_id: u32,
        offer_resources: ResourceQuantity,
        seek_resources: ResourceQuantity,
        owner_merchants_reserved: u8,
        accepting_merchants_used: u8,
        accepted_at: DateTime<Utc>,
    },
    /// Canonical accepting-village fact for marketplace acceptance materialization.
    ///
    /// This records the accepting village stock snapshot and busy merchants after
    /// reserving the outbound seeking-resources trip.
    MarketplaceOfferAcceptanceAppliedToVillage {
        offer_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        stocks: VillageStocks,
        busy_merchants: u8,
        applied_at: DateTime<Utc>,
    },
    BuildingConstructionScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
        cost: ResourceGroup,
        execute_at: DateTime<Utc>,
    },
    BuildingUpgradeScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
        cost: ResourceGroup,
        execute_at: DateTime<Utc>,
    },
    BuildingDowngradeScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
        execute_at: DateTime<Utc>,
    },
    BuildingAdded {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    },
    BuildingUpgraded {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    },
    BuildingDowngraded {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    },
    UnitTrainingScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        slot_id: u8,
        unit: UnitName,
        time_per_unit: i32,
        quantity_remaining: i32,
        cost: ResourceGroup,
        execute_at: DateTime<Utc>,
    },
    UnitTrained {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        unit: UnitName,
        quantity_trained: u32,
    },
    AcademyResearchScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        unit: UnitName,
        cost: ResourceGroup,
        execute_at: DateTime<Utc>,
    },
    AcademyResearchCompleted {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        unit: UnitName,
    },
    SmithyResearchScheduled {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        unit: UnitName,
        cost: ResourceGroup,
        execute_at: DateTime<Utc>,
    },
    SmithyResearchCompleted {
        action_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        unit: UnitName,
    },
}

impl fmt::Display for VillageEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            VillageEvent::VillageFounded { .. } => "VillageFounded",
            VillageEvent::VillageConquered { .. } => "VillageConquered",
            VillageEvent::VillageResourcesSet { .. } => "VillageResourcesSet",
            VillageEvent::VillageArmyDetached { .. } => "VillageArmyDetached",
            VillageEvent::HeroCreated { .. } => "HeroCreated",
            VillageEvent::HeroRevivalScheduled { .. } => "HeroRevivalScheduled",
            VillageEvent::HeroRevived { .. } => "HeroRevived",
            VillageEvent::ReinforcementSent { .. } => "ReinforcementSent",
            VillageEvent::ReinforcementArrived { .. } => "ReinforcementArrived",
            VillageEvent::ReinforcementAppliedToVillage { .. } => "ReinforcementAppliedToVillage",
            VillageEvent::ReinforcementsRecalled { .. } => "ReinforcementsRecalled",
            VillageEvent::ReinforcementsReleased { .. } => "ReinforcementsReleased",
            VillageEvent::SettlersSent { .. } => "SettlersSent",
            VillageEvent::SettlersArrived { .. } => "SettlersArrived",
            VillageEvent::AttackSent { .. } => "AttackSent",
            VillageEvent::AttackArrivalScheduled { .. } => "AttackArrivalScheduled",
            VillageEvent::AttackArrived { .. } => "AttackArrived",
            VillageEvent::AttackBattleResolved { .. } => "AttackBattleResolved",
            VillageEvent::BattleOutcomeAppliedToVillage { .. } => "BattleOutcomeAppliedToVillage",
            VillageEvent::ArmyReturned { .. } => "ArmyReturned",
            VillageEvent::ScoutSent { .. } => "ScoutSent",
            VillageEvent::ScoutArrived { .. } => "ScoutArrived",
            VillageEvent::ScoutBattleResolved { .. } => "ScoutBattleResolved",
            VillageEvent::MerchantsTripScheduled { .. } => "MerchantsTripScheduled",
            VillageEvent::MerchantsArrived { .. } => "MerchantsArrived",
            VillageEvent::MerchantTransferAppliedToVillage { .. } => {
                "MerchantTransferAppliedToVillage"
            }
            VillageEvent::MerchantsReturned { .. } => "MerchantsReturned",
            VillageEvent::MarketplaceOfferCreated { .. } => "MarketplaceOfferCreated",
            VillageEvent::MarketplaceOfferReservationAppliedToVillage { .. } => {
                "MarketplaceOfferReservationAppliedToVillage"
            }
            VillageEvent::MarketplaceOfferCanceled { .. } => "MarketplaceOfferCanceled",
            VillageEvent::MarketplaceOfferReservationReleasedFromVillage { .. } => {
                "MarketplaceOfferReservationReleasedFromVillage"
            }
            VillageEvent::MarketplaceOfferAccepted { .. } => "MarketplaceOfferAccepted",
            VillageEvent::MarketplaceOfferAcceptanceAppliedToVillage { .. } => {
                "MarketplaceOfferAcceptanceAppliedToVillage"
            }
            VillageEvent::BuildingConstructionScheduled { .. } => "BuildingConstructionScheduled",
            VillageEvent::BuildingUpgradeScheduled { .. } => "BuildingUpgradeScheduled",
            VillageEvent::BuildingDowngradeScheduled { .. } => "BuildingDowngradeScheduled",
            VillageEvent::BuildingAdded { .. } => "BuildingAdded",
            VillageEvent::BuildingUpgraded { .. } => "BuildingUpgraded",
            VillageEvent::BuildingDowngraded { .. } => "BuildingDowngraded",
            VillageEvent::UnitTrainingScheduled { .. } => "UnitTrainingScheduled",
            VillageEvent::UnitTrained { .. } => "UnitTrained",
            VillageEvent::AcademyResearchScheduled { .. } => "AcademyResearchScheduled",
            VillageEvent::AcademyResearchCompleted { .. } => "AcademyResearchCompleted",
            VillageEvent::SmithyResearchScheduled { .. } => "SmithyResearchScheduled",
            VillageEvent::SmithyResearchCompleted { .. } => "SmithyResearchCompleted",
        };
        f.write_str(name)
    }
}

impl EventPayload for VillageEvent {}
