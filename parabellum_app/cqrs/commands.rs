use uuid::Uuid;

use parabellum_game::{
    battle::{AttackType, ScoutingTarget},
    models::{army::TroopSet, map::MapQuadrant},
};
use parabellum_types::{
    army::UnitName,
    buildings::BuildingName,
    common::{Player, ResourceGroup},
    map::Position,
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
    pub attack_type: AttackType,
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
pub struct MarkReportRead {
    pub report_id: Uuid,
    pub player_id: Uuid,
}

impl Command for MarkReportRead {}

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

#[derive(Debug, Clone)]
pub struct RecallTroops {
    pub player_id: Uuid,
    pub village_id: u32,
    pub army_id: Uuid,
}

impl Command for RecallTroops {}

#[derive(Debug, Clone)]
pub struct ReleaseReinforcements {
    pub player_id: Uuid,
    pub village_id: u32,
    pub source_village_id: u32,
}

impl Command for ReleaseReinforcements {}
