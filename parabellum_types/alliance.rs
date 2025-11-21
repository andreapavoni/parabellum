use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Returns bitfield with standard officer permissions (invite, manage marks, send messages)
    /// Typically given to demoted leaders or trusted officers
    pub fn officer_permissions() -> i32 {
        (Self::InvitePlayer as i32) | (Self::ManageMarks as i32) | (Self::IgmMessage as i32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BonusType {
    Training = 1,
    Armor = 2,
    CombatPoints = 3,
    Trade = 4,
}

impl BonusType {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            1 => Some(BonusType::Training),
            2 => Some(BonusType::Armor),
            3 => Some(BonusType::CombatPoints),
            4 => Some(BonusType::Trade),
            _ => None,
        }
    }

    pub fn as_i16(self) -> i16 {
        self as i16
    }
}
