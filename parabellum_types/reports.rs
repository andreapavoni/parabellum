use serde::{Deserialize, Serialize};

use crate::common::ResourceGroup;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportPayload {
    Battle(BattleReportPayload),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattlePartyPayload {
    pub army_before: [u32; 10],
    pub survivors: [u32; 10],
    pub losses: [u32; 10],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleReportPayload {
    pub attacker_player: String,
    pub attacker_village: String,
    pub defender_player: String,
    pub defender_village: String,
    pub success: bool,
    pub bounty: ResourceGroup,
    #[serde(default)]
    pub attacker: Option<BattlePartyPayload>,
    pub defender: Option<BattlePartyPayload>,
    #[serde(default)]
    pub reinforcements: Vec<BattlePartyPayload>,
}
