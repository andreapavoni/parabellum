use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_core::GameError;
use parabellum_types::common::ResourceGroup;
pub use parabellum_types::alliance::{AlliancePermission, AllianceBonusType, DiplomacyType, DiplomacyStatus};

use crate::models::{player::Player, village::Village};

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
    pub max_members: i16,
    pub leader_id: Option<Uuid>,

    // Battle Statistics
    pub total_attack_points: i64,
    pub total_defense_points: i64,
    pub total_robber_points: i64,
    pub total_climber_points: i64,
    pub current_attack_points: i64,
    pub current_defense_points: i64,
    pub current_robber_points: i64,
    pub current_climber_points: i64,

    // Alliance Bonuses
    pub recruitment_bonus_level: i16,
    pub recruitment_bonus_contributions: i64,
    pub metallurgy_bonus_level: i16,
    pub metallurgy_bonus_contributions: i64,
    pub philosophy_bonus_level: i16,
    pub philosophy_bonus_contributions: i64,
    pub commerce_bonus_level: i16,
    pub commerce_bonus_contributions: i64,
}

impl Alliance {
    pub fn new(name: String, tag: String, max_members: i16, leader_id: Uuid) -> Result<Self, GameError> {
        if name.len() < 3 || name.len() > 20 {
            return Err(GameError::InvalidAllianceName("Length must be between 3 and 20 characters".to_string()));
        }
        if tag.len() < 3 || tag.len() > 5 {
            return Err(GameError::InvalidAllianceTag("Length must be between 3 and 5 characters".to_string()));
        }

        Ok(Self {
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
            total_robber_points: 0,
            total_climber_points: 0,
            current_attack_points: 0,
            current_defense_points: 0,
            current_robber_points: 0,
            current_climber_points: 0,
            recruitment_bonus_level: 0,
            recruitment_bonus_contributions: 0,
            metallurgy_bonus_level: 0,
            metallurgy_bonus_contributions: 0,
            philosophy_bonus_level: 0,
            philosophy_bonus_contributions: 0,
            commerce_bonus_level: 0,
            commerce_bonus_contributions: 0,
        })
    }

    /// Creates a new alliance with a founder player. The max_members is determined by the embassy level.
    pub fn create_with_founder(name: String, tag: String, embassy_level: u8, founder_id: Uuid) -> Result<Self, GameError> {
        Self::new(name, tag, embassy_level as i16, founder_id)
    }

    /// Returns the contributions needed to reach a specific level (cumulative).
    /// Based on original game formula with speed multiplier.
    pub fn get_bonus_contributions_needed(level: i16, speed: i32) -> i64 {
        let base_requirements = [
            0_i64,        // Level 0: 0
            2_400_000,    // Level 1: 2.4M
            19_200_000,   // Level 2: 19.2M
            38_400_000,   // Level 3: 38.4M
            76_800_000,   // Level 4: 76.8M
            153_600_000,  // Level 5: 153.6M
        ];

        let rate = if speed > 20 {
            (speed as f64 / 1000.0).ceil() as i64
        } else {
            1
        };

        if level < 0 || level >= base_requirements.len() as i16 {
            return 0;
        }

        // Return cumulative sum up to this level
        let mut total = 0;
        for i in 0..=level as usize {
            total += base_requirements[i];
        }
        total * rate
    }

    /// Calculates the current bonus level based on contributions.
    pub fn calculate_bonus_level(contributions: i64, speed: i32) -> i16 {
        let mut level = 0;
        for i in 1..=5 {
            if contributions >= Self::get_bonus_contributions_needed(i, speed) {
                level = i;
            }
        }
        level
    }

    /// Returns the donation limit per player for a given embassy level.
    pub fn get_donation_limit(embassy_level: u8, speed: i32) -> i64 {
        let limits = [
            300_000_i64,  // Level 0-1
            400_000,      // Level 1
            550_000,      // Level 2
            750_000,      // Level 3
            1_000_000,    // Level 4
            1_000_000,    // Level 5+
        ];

        let level_index = embassy_level.min(5) as usize;
        limits[level_index] * speed as i64
    }

    /// Returns the cooldown in seconds for new players joining an alliance with a given bonus level.
    /// Players joining an alliance with bonus level 3+ have a cooldown before they can benefit or contribute.
    pub fn get_new_player_cooldown(bonus_level: i16, speed: i32) -> i64 {
        if bonus_level <= 2 {
            return 0;
        }
        let days = bonus_level - 2;
        ((days as i32 * 86400) as f64 / speed as f64).ceil() as i64
    }

    /// Returns the upgrade duration in seconds for upgrading to the next level.
    pub fn get_bonus_upgrade_duration(current_level: i16, speed: i32) -> i64 {
        let target_level = current_level + 1;
        ((target_level as i32 * 86400) as f64 / speed as f64).round() as i64
    }

    pub fn get_bonus_upgrade_cost(&self, level: i16) -> i64 {
        // Use the realistic contribution requirements
        Self::get_bonus_contributions_needed(level, 1) // Default speed of 1 for cost calculation
    }

    /// Returns the current level for a given bonus type.
    pub fn get_bonus_level(&self, bonus_type: AllianceBonusType) -> i16 {
        match bonus_type {
            AllianceBonusType::Recruitment => self.recruitment_bonus_level,
            AllianceBonusType::Metallurgy => self.metallurgy_bonus_level,
            AllianceBonusType::Philosophy => self.philosophy_bonus_level,
            AllianceBonusType::Commerce => self.commerce_bonus_level,
        }
    }

    /// Returns the current contributions for a given bonus type.
    pub fn get_bonus_contributions(&self, bonus_type: AllianceBonusType) -> i64 {
        match bonus_type {
            AllianceBonusType::Recruitment => self.recruitment_bonus_contributions,
            AllianceBonusType::Metallurgy => self.metallurgy_bonus_contributions,
            AllianceBonusType::Philosophy => self.philosophy_bonus_contributions,
            AllianceBonusType::Commerce => self.commerce_bonus_contributions,
        }
    }

    /// Calculates contribution points from resources (1000 resources = 1 point).
    pub fn calculate_contribution_points(resources: &ResourceGroup) -> i64 {
        (resources.total() / 1000) as i64
    }

    /// Checks if a bonus can be upgraded based on current contributions.
    pub fn can_upgrade_bonus(&self, bonus_type: AllianceBonusType) -> bool {
        let level = self.get_bonus_level(bonus_type);
        let contributions = self.get_bonus_contributions(bonus_type);
        let next_level_cost = self.get_bonus_upgrade_cost(level + 1);
        contributions >= next_level_cost
    }

    /// Returns the upgrade duration in seconds for upgrading the current bonus (not adjusted for speed).
    /// This is the base duration before applying game speed multiplier.
    /// Returns None if the level is already at max (5).
    pub fn get_upgrade_duration_seconds(&self, bonus_type: AllianceBonusType) -> Option<i64> {
        let current_level = self.get_bonus_level(bonus_type);
        if current_level >= 5 {
            return None;
        }
        Some(Self::get_bonus_upgrade_duration(current_level, 1))
    }

    /// Adds a contribution to a specific bonus type, deducts resources from village, and updates player stats.
    /// Validates donation limits and cooldowns before accepting contribution.
    /// Returns the contribution points added and whether an upgrade was triggered.
    pub fn add_contribution(
        &mut self,
        bonus_type: AllianceBonusType,
        resources: &ResourceGroup,
        village: &mut Village,
        player: &mut Player,
        embassy_level: u8,
        speed: i32,
        current_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<ContributionResult, GameError> {
        // Calculate contribution points first (before deducting resources)
        let contribution_points = Self::calculate_contribution_points(resources);

        if contribution_points == 0 {
            return Ok(ContributionResult {
                contribution_points: 0,
                upgrade_triggered: false,
            });
        }

        // Check donation limit based on embassy level
        let donation_limit = Self::get_donation_limit(embassy_level, speed);
        let current_contributions = player.get_alliance_contribution(bonus_type);

        if current_contributions + contribution_points > donation_limit {
            return Err(GameError::AllianceDonationLimitExceeded);
        }

        // Check new player cooldown
        if let Some(join_time) = player.alliance_join_time {
            let current_bonus_level = self.get_bonus_level(bonus_type);
            let cooldown_seconds = Self::get_new_player_cooldown(current_bonus_level, speed);

            if cooldown_seconds > 0 {
                let time_since_join = (current_time - join_time).num_seconds();
                if time_since_join < cooldown_seconds as i64 {
                    return Err(GameError::AllianceNewPlayerCooldown);
                }
            }
        }

        // Deduct resources from village
        village.deduct_resources(resources)?;

        // Update player stats for the specific bonus type
        player.add_alliance_contribution(bonus_type, contribution_points);

        // Update alliance bonus contributions
        match bonus_type {
            AllianceBonusType::Recruitment => self.recruitment_bonus_contributions += contribution_points,
            AllianceBonusType::Metallurgy => self.metallurgy_bonus_contributions += contribution_points,
            AllianceBonusType::Philosophy => self.philosophy_bonus_contributions += contribution_points,
            AllianceBonusType::Commerce => self.commerce_bonus_contributions += contribution_points,
        }

        // Check if upgrade should be triggered
        let upgrade_triggered = self.can_upgrade_bonus(bonus_type);

        Ok(ContributionResult {
            contribution_points,
            upgrade_triggered,
        })
    }

    /// Upgrades the bonus level for a given bonus type.
    pub fn upgrade_bonus(&mut self, bonus_type: AllianceBonusType) -> Result<(), GameError> {
        match bonus_type {
            AllianceBonusType::Recruitment => self.recruitment_bonus_level += 1,
            AllianceBonusType::Metallurgy => self.metallurgy_bonus_level += 1,
            AllianceBonusType::Philosophy => self.philosophy_bonus_level += 1,
            AllianceBonusType::Commerce => self.commerce_bonus_level += 1,
        }
        Ok(())
    }

    /// Returns the metallurgy bonus as a multiplier (e.g., level 3 = 0.03 or 3%).
    pub fn get_metallurgy_bonus_multiplier(&self) -> f64 {
        self.metallurgy_bonus_level as f64 * 0.01
    }

    /// Returns the recruitment bonus as a multiplier (e.g., level 2 = 0.02 or 2%).
    pub fn get_recruitment_bonus_multiplier(&self) -> f64 {
        self.recruitment_bonus_level as f64 * 0.01
    }

    /// Returns the philosophy bonus as a multiplier (e.g., level 1 = 0.01 or 1%).
    pub fn get_philosophy_bonus_multiplier(&self) -> f64 {
        self.philosophy_bonus_level as f64 * 0.01
    }

    /// Returns the commerce bonus as a multiplier (e.g., level 4 = 0.04 or 4%).
    pub fn get_commerce_bonus_multiplier(&self) -> f64 {
        self.commerce_bonus_level as f64 * 0.01
    }

    /// Checks if the given player is the alliance leader.
    pub fn is_leader(&self, player_id: Uuid) -> bool {
        self.leader_id == Some(player_id)
    }

    /// Transfers leadership from current leader to new leader.
    /// Validates that the executor is the current leader and the new leader is not already leader.
    pub fn transfer_leadership(
        &mut self,
        executor_id: Uuid,
        new_leader_id: Uuid,
    ) -> Result<(), GameError> {
        // Verify executor is the current leader
        if !self.is_leader(executor_id) {
            return Err(GameError::NotAllianceLeader);
        }

        // Verify new leader is not already the leader
        if self.is_leader(new_leader_id) {
            return Err(GameError::PlayerAlreadyLeader);
        }

        // Transfer leadership
        self.leader_id = Some(new_leader_id);
        Ok(())
    }
}

/// Result of a contribution operation.
#[derive(Debug, Clone)]
pub struct ContributionResult {
    pub contribution_points: i64,
    pub upgrade_triggered: bool,
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
    DiplomacyProposed = 6,
    DiplomacyAccepted = 7,
    DiplomacyDeclined = 8,
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
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl AllianceLog {
    pub fn new(alliance_id: Uuid, type_: AllianceLogType, data: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            alliance_id,
            type_: type_.as_i16(),
            data,
            created_at: chrono::Utc::now(),
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

impl AllianceDiplomacy {
    /// Creates a new alliance diplomacy proposal
    pub fn new(alliance1_id: Uuid, alliance2_id: Uuid, diplomacy_type: DiplomacyType) -> Self {
        Self {
            id: Uuid::new_v4(),
            alliance1_id,
            alliance2_id,
            type_: diplomacy_type as i16,
            accepted: DiplomacyStatus::Pending as i16,
        }
    }

    /// Accepts the diplomacy proposal
    pub fn accept(&mut self) {
        self.accepted = DiplomacyStatus::Accepted as i16;
    }

    /// Declines the diplomacy proposal
    pub fn decline(&mut self) {
        self.accepted = DiplomacyStatus::Declined as i16;
    }

    /// Checks if the diplomacy is pending
    pub fn is_pending(&self) -> bool {
        self.accepted == DiplomacyStatus::Pending as i16
    }

    /// Checks if the diplomacy is accepted
    pub fn is_accepted(&self) -> bool {
        self.accepted == DiplomacyStatus::Accepted as i16
    }

    /// Checks if the diplomacy is declined
    pub fn is_declined(&self) -> bool {
        self.accepted == DiplomacyStatus::Declined as i16
    }

    /// Gets the diplomacy type
    pub fn get_type(&self) -> Option<DiplomacyType> {
        match self.type_ {
            1 => Some(DiplomacyType::War),
            2 => Some(DiplomacyType::NAP),
            3 => Some(DiplomacyType::Alliance),
            _ => None,
        }
    }
}

/// Verifies that a player has the specified permission.
pub fn verify_permission(player: &Player, permission: AlliancePermission) -> Result<(), GameError> {
    if !AlliancePermission::has_permission(player.alliance_role.unwrap_or(0), permission) {
        return Err(match permission {
            AlliancePermission::InvitePlayer => GameError::NoInvitePermission,
            AlliancePermission::KickPlayer => GameError::NoKickPermission,
            AlliancePermission::ManageMarks => GameError::NoManageMarksPermission,
            AlliancePermission::AllianceDiplomacy => GameError::NoDiplomacyPermission,
            _ => GameError::NoInvitePermission, // Generic fallback
        });
    }
    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_metallurgy_bonus_multiplier() {
        let mut alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            60,
            Uuid::new_v4(),
        ).unwrap();

        // Test level 0
        alliance.metallurgy_bonus_level = 0;
        assert_eq!(alliance.get_metallurgy_bonus_multiplier(), 0.0);

        // Test level 1 = 1%
        alliance.metallurgy_bonus_level = 1;
        assert_eq!(alliance.get_metallurgy_bonus_multiplier(), 0.01);

        // Test level 3 = 3%
        alliance.metallurgy_bonus_level = 3;
        assert_eq!(alliance.get_metallurgy_bonus_multiplier(), 0.03);

        // Test level 5 = 5%
        alliance.metallurgy_bonus_level = 5;
        assert_eq!(alliance.get_metallurgy_bonus_multiplier(), 0.05);
    }

    #[test]
    fn test_get_recruitment_bonus_multiplier() {
        let mut alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            60,
            Uuid::new_v4(),
        ).unwrap();

        // Test level 0
        alliance.recruitment_bonus_level = 0;
        assert_eq!(alliance.get_recruitment_bonus_multiplier(), 0.0);

        // Test level 2 = 2%
        alliance.recruitment_bonus_level = 2;
        assert_eq!(alliance.get_recruitment_bonus_multiplier(), 0.02);

        // Test level 4 = 4%
        alliance.recruitment_bonus_level = 4;
        assert_eq!(alliance.get_recruitment_bonus_multiplier(), 0.04);

        // Test level 5 = 5%
        alliance.recruitment_bonus_level = 5;
        assert_eq!(alliance.get_recruitment_bonus_multiplier(), 0.05);
    }

    #[test]
    fn test_get_philosophy_bonus_multiplier() {
        let mut alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            60,
            Uuid::new_v4(),
        ).unwrap();

        // Test level 0
        alliance.philosophy_bonus_level = 0;
        assert_eq!(alliance.get_philosophy_bonus_multiplier(), 0.0);

        // Test level 1 = 1%
        alliance.philosophy_bonus_level = 1;
        assert_eq!(alliance.get_philosophy_bonus_multiplier(), 0.01);

        // Test level 3 = 3%
        alliance.philosophy_bonus_level = 3;
        assert_eq!(alliance.get_philosophy_bonus_multiplier(), 0.03);
    }

    #[test]
    fn test_get_commerce_bonus_multiplier() {
        let mut alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            60,
            Uuid::new_v4(),
        ).unwrap();

        // Test level 0
        alliance.commerce_bonus_level = 0;
        assert_eq!(alliance.get_commerce_bonus_multiplier(), 0.0);

        // Test level 2 = 2%
        alliance.commerce_bonus_level = 2;
        assert_eq!(alliance.get_commerce_bonus_multiplier(), 0.02);

        // Test level 4 = 4%
        alliance.commerce_bonus_level = 4;
        assert_eq!(alliance.get_commerce_bonus_multiplier(), 0.04);

        // Test level 5 = 5%
        alliance.commerce_bonus_level = 5;
        assert_eq!(alliance.get_commerce_bonus_multiplier(), 0.05);
    }

    #[test]
    fn test_all_bonus_multipliers_together() {
        let mut alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            60,
            Uuid::new_v4(),
        ).unwrap();

        // Set all bonuses to different levels
        alliance.metallurgy_bonus_level = 3;
        alliance.recruitment_bonus_level = 2;
        alliance.philosophy_bonus_level = 1;
        alliance.commerce_bonus_level = 4;

        // Verify each returns correct multiplier
        assert_eq!(alliance.get_metallurgy_bonus_multiplier(), 0.03);
        assert_eq!(alliance.get_recruitment_bonus_multiplier(), 0.02);
        assert_eq!(alliance.get_philosophy_bonus_multiplier(), 0.01);
        assert_eq!(alliance.get_commerce_bonus_multiplier(), 0.04);
    }

    // ========== AllianceDiplomacy Domain Tests ==========

    #[test]
    fn test_alliance_diplomacy_new_creates_pending_status() {
        let alliance1_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

        let diplomacy = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::NAP);

        assert_eq!(diplomacy.alliance1_id, alliance1_id);
        assert_eq!(diplomacy.alliance2_id, alliance2_id);
        assert_eq!(diplomacy.type_, DiplomacyType::NAP as i16);
        assert!(diplomacy.is_pending());
        assert!(!diplomacy.is_accepted());
        assert!(!diplomacy.is_declined());
    }

    #[test]
    fn test_alliance_diplomacy_new_with_different_types() {
        let alliance1_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

        // Test NAP
        let nap = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::NAP);
        assert_eq!(nap.get_type(), Some(DiplomacyType::NAP));

        // Test Alliance
        let alliance = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::Alliance);
        assert_eq!(alliance.get_type(), Some(DiplomacyType::Alliance));

        // Test War
        let war = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::War);
        assert_eq!(war.get_type(), Some(DiplomacyType::War));
    }

    #[test]
    fn test_alliance_diplomacy_accept_from_pending() {
        let alliance1_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

        let mut diplomacy = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::NAP);
        assert!(diplomacy.is_pending());

        diplomacy.accept();

        assert!(diplomacy.is_accepted());
        assert!(!diplomacy.is_pending());
        assert!(!diplomacy.is_declined());
    }

    #[test]
    fn test_alliance_diplomacy_decline_from_pending() {
        let alliance1_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

        let mut diplomacy = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::Alliance);
        assert!(diplomacy.is_pending());

        diplomacy.decline();

        assert!(diplomacy.is_declined());
        assert!(!diplomacy.is_pending());
        assert!(!diplomacy.is_accepted());
    }

    #[test]
    fn test_alliance_diplomacy_accept_is_idempotent() {
        let alliance1_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

        let mut diplomacy = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::NAP);

        // Accept once
        diplomacy.accept();
        assert!(diplomacy.is_accepted());

        // Accept again - should remain accepted
        diplomacy.accept();
        assert!(diplomacy.is_accepted());
    }

    #[test]
    fn test_alliance_diplomacy_decline_is_idempotent() {
        let alliance1_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

        let mut diplomacy = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::NAP);

        // Decline once
        diplomacy.decline();
        assert!(diplomacy.is_declined());

        // Decline again - should remain declined
        diplomacy.decline();
        assert!(diplomacy.is_declined());
    }

    #[test]
    fn test_alliance_diplomacy_can_change_from_accepted_to_declined() {
        let alliance1_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

        let mut diplomacy = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::Alliance);

        // Accept first
        diplomacy.accept();
        assert!(diplomacy.is_accepted());

        // Then decline (e.g., alliance broke the agreement)
        diplomacy.decline();
        assert!(diplomacy.is_declined());
        assert!(!diplomacy.is_accepted());
    }

    #[test]
    fn test_alliance_diplomacy_can_change_from_declined_to_accepted() {
        let alliance1_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

        let mut diplomacy = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::NAP);

        // Decline first
        diplomacy.decline();
        assert!(diplomacy.is_declined());

        // Then accept (e.g., reconsidered the proposal)
        diplomacy.accept();
        assert!(diplomacy.is_accepted());
        assert!(!diplomacy.is_declined());
    }

    #[test]
    fn test_alliance_diplomacy_status_checks_are_mutually_exclusive() {
        let alliance1_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

        // Pending state
        let pending = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::NAP);
        assert!(pending.is_pending() && !pending.is_accepted() && !pending.is_declined());

        // Accepted state
        let mut accepted = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::NAP);
        accepted.accept();
        assert!(!accepted.is_pending() && accepted.is_accepted() && !accepted.is_declined());

        // Declined state
        let mut declined = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::NAP);
        declined.decline();
        assert!(!declined.is_pending() && !declined.is_accepted() && declined.is_declined());
    }
}

