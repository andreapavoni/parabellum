use serde::{Deserialize, Serialize};

use crate::{
    battle::{AttackType, BuildingDamageReport, ScoutingBattleReport},
    common::ResourceGroup,
    map::Position,
    tribe::Tribe,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReinforcementReportPayload {
    pub sender_player: String,
    pub sender_village: String,
    pub sender_position: Position,
    pub receiver_player: String,
    pub receiver_village: String,
    pub receiver_position: Position,
    pub tribe: Tribe,
    pub units: [u32; 10],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportPayload {
    Battle(BattleReportPayload),
    Reinforcement(ReinforcementReportPayload),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattlePartyPayload {
    pub tribe: Tribe,
    pub army_before: [u32; 10],
    pub survivors: [u32; 10],
    pub losses: [u32; 10],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleReportPayload {
    pub attack_type: AttackType,
    pub attacker_player: String,
    pub attacker_village: String,
    pub attacker_position: Position,
    pub defender_player: String,
    pub defender_village: String,
    pub defender_position: Position,
    pub success: bool,
    pub bounty: ResourceGroup,
    #[serde(default)]
    pub attacker: Option<BattlePartyPayload>,
    pub defender: Option<BattlePartyPayload>,
    #[serde(default)]
    pub reinforcements: Vec<BattlePartyPayload>,
    #[serde(default)]
    pub scouting: Option<ScoutingBattleReport>,
    #[serde(default)]
    pub wall_damage: Option<BuildingDamageReport>,
    #[serde(default)]
    pub catapult_damage: Vec<BuildingDamageReport>,
}
