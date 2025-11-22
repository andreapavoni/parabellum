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
    pub fn has_permission(role_bitfield: i16, permission: AlliancePermission) -> bool {
        (role_bitfield & (permission as i16)) != 0
    }

    /// Returns bitfield with all permissions enabled (255)
    pub fn all_permissions() -> i16 {
        255
    }

    /// Returns bitfield with standard officer permissions (invite, manage marks, send messages)
    /// Typically given to demoted leaders or trusted officers
    pub fn officer_permissions() -> i16 {
        (Self::InvitePlayer as i16) | (Self::ManageMarks as i16) | (Self::IgmMessage as i16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllianceBonusType {
    Recruitment = 1,
    Metallurgy = 2,
    Philosophy = 3,
    Commerce = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiplomacyType {
    War = 1,
    NAP = 2,  // Non-Aggression Pact
    Alliance = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiplomacyStatus {
    Declined = -1,
    Pending = 0,
    Accepted = 1,
}
