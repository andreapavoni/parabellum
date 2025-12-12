use serde::{Deserialize, Serialize};

use crate::{buildings::BuildingName, common::ResourceGroup};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub enum AttackType {
    Raid,   // Raid
    Normal, // Attack / Siege / Conquer
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub enum ScoutingTarget {
    Resources,
    Defenses,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub enum ScoutingTargetReport {
    Resources(ResourceGroup),
    Defenses {
        wall: Option<u8>,
        palace: Option<u8>,
        residence: Option<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoutingBattleReport {
    pub was_detected: bool,
    pub target: ScoutingTarget,
    pub target_report: ScoutingTargetReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingDamageReport {
    pub name: BuildingName,
    pub level_before: u8,
    pub level_after: u8,
}
