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
pub enum AllianceBonusType {
    Recruitment = 1,
    Metallurgy = 2,
    Philosophy = 3,
    Commerce = 4,
}
