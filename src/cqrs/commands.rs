use uuid::Uuid;

use crate::{
    cqrs::Command,
    game::models::{
        Player, Tribe,
        army::UnitName,
        buildings::BuildingName,
        map::{MapQuadrant, Position},
    },
};

#[derive(Debug, Clone)]
pub struct AddBuilding {
    pub player_id: Uuid,
    pub village_id: u32,
    pub slot_id: u8,
    pub name: BuildingName,
}

impl Command for AddBuilding {}

#[derive(Debug, Clone)]
pub struct AttackVillage {
    pub player_id: Uuid,
    pub village_id: u32,
    pub army_id: Uuid,
    pub target_village_id: u32,
    pub catapult_targets: [BuildingName; 2],
}

impl Command for AttackVillage {}

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
    pub tribe: Tribe,
}

impl RegisterPlayer {
    pub fn new(id: Option<Uuid>, username: String, tribe: Tribe) -> Self {
        Self {
            id: id.unwrap_or(Uuid::new_v4()),
            username,
            tribe,
        }
    }
}

impl Command for RegisterPlayer {}

#[derive(Debug, Clone)]
pub struct RegisterVillage {
    pub player: Player,
    pub quadrant: MapQuadrant,
}

impl RegisterVillage {
    pub fn new(player: Player, quadrant: MapQuadrant) -> Self {
        Self { player, quadrant }
    }
}

impl Command for RegisterVillage {}

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
}

impl Command for TrainUnits {}
