use uuid::Uuid;

use parabellum_game::{
    battle::ScoutingTarget,
    models::{army::TroopSet, map::MapQuadrant, player::Player},
};
use parabellum_types::{
    alliance::AllianceBonusType,
    army::UnitName,
    buildings::BuildingName,
    common::ResourceGroup,
    map::Position,
    map_flag::MapFlagType,
    tribe::Tribe,
};

use crate::cqrs::Command;

#[derive(Debug, Clone)]
pub struct AddBuilding {
    pub player_id: Uuid,
    pub village_id: u32,
    pub slot_id: u8,
    pub name: BuildingName,
}

impl Command for AddBuilding {}

#[derive(Debug, Clone)]
pub struct UpgradeBuilding {
    pub player_id: Uuid,
    pub village_id: u32,
    pub slot_id: u8,
}

impl Command for UpgradeBuilding {}

#[derive(Debug, Clone)]
pub struct DowngradeBuilding {
    pub player_id: Uuid,
    pub village_id: u32,
    pub slot_id: u8,
}

impl Command for DowngradeBuilding {}

#[derive(Debug, Clone)]
pub struct AttackVillage {
    pub player_id: Uuid,
    pub village_id: u32,
    pub army_id: Uuid,
    pub units: TroopSet,
    pub target_village_id: u32,
    pub catapult_targets: [BuildingName; 2],
    pub hero_id: Option<Uuid>,
}

impl Command for AttackVillage {}

#[derive(Debug, Clone)]
pub struct ScoutVillage {
    pub player_id: Uuid,
    pub village_id: u32,
    pub army_id: Uuid,
    pub units: TroopSet,
    pub target_village_id: u32,
    pub target: ScoutingTarget,
}
impl Command for ScoutVillage {}

#[derive(Debug, Clone)]
pub struct ReinforceVillage {
    pub player_id: Uuid,
    pub village_id: u32,
    pub army_id: Uuid,
    pub units: TroopSet,
    pub target_village_id: u32,
    pub hero_id: Option<Uuid>,
}
impl Command for ReinforceVillage {}

#[derive(Clone)]
pub struct FoundVillage {
    pub player: Player,
    pub position: Position,
}

impl FoundVillage {
    pub fn new(player: Player, position: Position) -> Self {
        Self { player, position }
    }
}

impl Command for FoundVillage {}

#[derive(Debug, Clone)]
pub struct RegisterPlayer {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password: String,
    pub tribe: Tribe,
    pub quadrant: MapQuadrant,
}

impl RegisterPlayer {
    pub fn new(
        username: String,
        email: String,
        password: String,
        tribe: Tribe,
        quadrant: MapQuadrant,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            tribe,
            quadrant,
            email,
            password,
        }
    }
}

impl Command for RegisterPlayer {}

#[derive(Debug, Clone)]
pub struct ResearchAcademy {
    pub unit: UnitName,
    pub village_id: u32,
}

impl Command for ResearchAcademy {}

#[derive(Debug, Clone)]
pub struct ResearchSmithy {
    pub unit: UnitName,
    pub village_id: u32,
}

impl Command for ResearchSmithy {}

#[derive(Debug, Clone)]
pub struct TrainUnits {
    pub player_id: Uuid,
    pub village_id: u32,
    pub unit_idx: u8,
    pub quantity: i32,
    pub building_name: BuildingName,
}

impl Command for TrainUnits {}

#[derive(Debug, Clone)]
pub struct SendResources {
    pub village_id: u32,
    pub player_id: Uuid,
    pub target_village_id: u32,
    pub resources: ResourceGroup,
}
impl Command for SendResources {}

#[derive(Debug, Clone)]
pub struct CreateMarketplaceOffer {
    pub village_id: u32,
    pub offer_resources: ResourceGroup,
    pub seek_resources: ResourceGroup,
}
impl Command for CreateMarketplaceOffer {}

#[derive(Debug, Clone)]
pub struct AcceptMarketplaceOffer {
    pub player_id: Uuid,
    pub village_id: u32,
    pub offer_id: Uuid,
}
impl Command for AcceptMarketplaceOffer {}

#[derive(Debug, Clone)]
pub struct CreateHero {
    pub id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
}

impl CreateHero {
    pub fn new(id: Option<Uuid>, player_id: Uuid, village_id: u32) -> Self {
        Self {
            id: id.unwrap_or_else(Uuid::new_v4),
            player_id,
            village_id,
        }
    }
}

impl Command for CreateHero {}

#[derive(Debug, Clone)]
pub struct ReviveHero {
    pub player_id: Uuid,
    pub hero_id: Uuid,
    pub village_id: u32,
    pub reset: bool,
}

impl Command for ReviveHero {}

// Alliance Commands

#[derive(Debug, Clone)]
pub struct CreateAlliance {
    pub player_id: Uuid,
    pub name: String,
    pub tag: String,
}

impl Command for CreateAlliance {}

#[derive(Debug, Clone)]
pub struct InviteToAlliance {
    pub player_id: Uuid,
    pub alliance_id: Uuid,
    pub target_player_id: Uuid,
}

impl Command for InviteToAlliance {}

#[derive(Debug, Clone)]
pub struct AcceptAllianceInvite {
    pub player_id: Uuid,
    pub alliance_id: Uuid,
}

impl Command for AcceptAllianceInvite {}

#[derive(Debug, Clone)]
pub struct LeaveAlliance {
    pub player_id: Uuid,
}

impl Command for LeaveAlliance {}

#[derive(Debug, Clone)]
pub struct KickFromAlliance {
    pub player_id: Uuid,
    pub alliance_id: Uuid,
    pub target_player_id: Uuid,
}

impl Command for KickFromAlliance {}

#[derive(Debug, Clone)]
pub struct SetAllianceLeader {
    pub player_id: Uuid,
    pub alliance_id: Uuid,
    pub new_leader_id: Uuid,
}

impl Command for SetAllianceLeader {}

#[derive(Debug, Clone)]
pub struct ContributeToAllianceBonus {
    pub player_id: Uuid,
    pub village_id: u32,
    pub alliance_id: Uuid,
    pub bonus_type: AllianceBonusType,
    pub resources: ResourceGroup,
}

impl Command for ContributeToAllianceBonus {}

// Map Flag Commands

#[derive(Debug, Clone)]
pub struct CreateCustomFlag {
    pub player_id: Uuid,
    pub alliance_id: Option<Uuid>,  // None for player-owned, Some for alliance-owned
    pub x: i32,
    pub y: i32,
    pub color: i16,
    pub text: String,
}

impl Command for CreateCustomFlag {}

#[derive(Debug, Clone)]
pub struct CreateMultiMark {
    pub player_id: Uuid,
    pub alliance_id: Option<Uuid>,  // None for player-owned, Some for alliance-owned
    pub target_id: Uuid,  // Target player or alliance ID
    pub flag_type: MapFlagType,  // PlayerMark or AllianceMark
    pub color: i16,
}

impl Command for CreateMultiMark {}

#[derive(Debug, Clone)]
pub struct UpdateMapFlag {
    pub player_id: Uuid,
    pub alliance_id: Option<Uuid>,
    pub flag_id: Uuid,
    pub color: i16,
    pub text: Option<String>,
}

impl Command for UpdateMapFlag {}

#[derive(Debug, Clone)]
pub struct DeleteMapFlag {
    pub player_id: Uuid,
    pub alliance_id: Option<Uuid>,
    pub flag_id: Uuid,
}

impl Command for DeleteMapFlag {}

