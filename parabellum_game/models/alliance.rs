use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alliance {
    pub id: Uuid,
    pub name: String,
    pub tag: String,
    pub desc1: Option<String>,
    pub desc2: Option<String>,
    pub info1: Option<String>,
    pub info2: Option<String>,
    pub forum_link: Option<String>,
    pub max_members: i32,
    pub leader_id: Option<Uuid>,

    // Battle Statistics
    pub total_attack_points: i64,
    pub total_defense_points: i64,
    pub current_attack_points: i64,
    pub current_defense_points: i64,
    pub current_robber_points: i64,

    // Alliance Bonuses
    pub training_bonus_level: i32,
    pub training_bonus_contributions: i64,
    pub armor_bonus_level: i32,
    pub armor_bonus_contributions: i64,
    pub cp_bonus_level: i32,
    pub cp_bonus_contributions: i64,
    pub trade_bonus_level: i32,
    pub trade_bonus_contributions: i64,

    pub old_pop: i32,
}

impl Alliance {
    pub fn new(name: String, tag: String, max_members: i32, leader_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            tag,
            desc1: Some(String::new()),
            desc2: Some(String::new()),
            info1: Some(String::new()),
            info2: Some(String::new()),
            forum_link: None,
            max_members,
            leader_id: Some(leader_id),
            total_attack_points: 0,
            total_defense_points: 0,
            current_attack_points: 0,
            current_defense_points: 0,
            current_robber_points: 0,
            training_bonus_level: 0,
            training_bonus_contributions: 0,
            armor_bonus_level: 0,
            armor_bonus_contributions: 0,
            cp_bonus_level: 0,
            cp_bonus_contributions: 0,
            trade_bonus_level: 0,
            trade_bonus_contributions: 0,
            old_pop: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceInvite {
    pub id: Uuid,
    pub from_player_id: Uuid,
    pub alliance_id: Uuid,
    pub to_player_id: Uuid,
}

impl AllianceInvite {
    pub fn new(from_player_id: Uuid, alliance_id: Uuid, to_player_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_player_id,
            alliance_id,
            to_player_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllianceLogType {
    AllianceCreated = 1,
    PlayerJoined = 2,
    PlayerLeft = 3,
    PlayerKicked = 4,
    RoleChanged = 5,
}

impl AllianceLogType {
    pub fn as_i16(self) -> i16 {
        self as i16
    }

    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            1 => Some(AllianceLogType::AllianceCreated),
            2 => Some(AllianceLogType::PlayerJoined),
            3 => Some(AllianceLogType::PlayerLeft),
            4 => Some(AllianceLogType::PlayerKicked),
            5 => Some(AllianceLogType::RoleChanged),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceLog {
    pub id: Uuid,
    pub alliance_id: Uuid,
    #[serde(rename = "type")]
    pub type_: i16,
    pub data: Option<String>,
    pub time: i32,
}

impl AllianceLog {
    pub fn new(alliance_id: Uuid, type_: AllianceLogType, data: Option<String>, time: i32) -> Self {
        Self {
            id: Uuid::new_v4(),
            alliance_id,
            type_: type_.as_i16(),
            data,
            time,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceDiplomacy {
    pub id: Uuid,
    pub alliance1_id: Uuid,
    pub alliance2_id: Uuid,
    #[serde(rename = "type")]
    pub type_: i16,
    pub accepted: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlliancePermission {
    AssignToPosition = 1,
    KickPlayer = 2,
    ChangeAllianceDesc = 4,
    AllianceDiplomacy = 8,
    IgmMessage = 16,
    InvitePlayer = 32,
    ManageForum = 64,
    ManageMarks = 128,
}

impl AlliancePermission {
    pub fn has_permission(role_bitfield: i32, permission: AlliancePermission) -> bool {
        (role_bitfield & (permission as i32)) != 0
    }
    
    /// Returns bitfield with all permissions enabled (255)
    pub fn all_permissions() -> i32 {
        255
    }
}

