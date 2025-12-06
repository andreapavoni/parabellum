use serde::{Deserialize, Serialize};

use crate::common::ResourceGroup;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportPayload {
    Battle(BattleReportPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleReportPayload {
    pub attacker_player: String,
    pub attacker_village: String,
    pub defender_player: String,
    pub defender_village: String,
    pub success: bool,
    pub bounty: ResourceGroup,
}
