use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_core::GameError;
use parabellum_types::tribe::Tribe;
pub use parabellum_types::alliance::AllianceBonusType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
    pub tribe: Tribe,
    pub user_id: Uuid,
    pub alliance_id: Option<Uuid>,
    pub alliance_role: Option<i16>,
    pub alliance_join_time: Option<chrono::DateTime<chrono::Utc>>,
    pub current_alliance_recruitment_contributions: i64,
    pub current_alliance_metallurgy_contributions: i64,
    pub current_alliance_philosophy_contributions: i64,
    pub current_alliance_commerce_contributions: i64,
    pub total_alliance_recruitment_contributions: i64,
    pub total_alliance_metallurgy_contributions: i64,
    pub total_alliance_philosophy_contributions: i64,
    pub total_alliance_commerce_contributions: i64,
}

impl Player {
    pub fn join_alliance(&mut self, alliance_id: Uuid, role: i16) -> Result<(), GameError> {
        if self.alliance_id.is_some() {
            return Err(GameError::PlayerAlreadyInAlliance);
        }
        self.alliance_id = Some(alliance_id);
        self.alliance_role = Some(role);
        self.alliance_join_time = Some(chrono::Utc::now());
        Ok(())
    }

    pub fn leave_alliance(&mut self) {
        self.alliance_id = None;
        self.alliance_role = None;
        self.alliance_join_time = None;

        // Reset contributions
        self.current_alliance_recruitment_contributions = 0;
        self.current_alliance_metallurgy_contributions = 0;
        self.current_alliance_philosophy_contributions = 0;
        self.current_alliance_commerce_contributions = 0;
        self.total_alliance_recruitment_contributions = 0;
        self.total_alliance_metallurgy_contributions = 0;
        self.total_alliance_philosophy_contributions = 0;
        self.total_alliance_commerce_contributions = 0;
    }

    pub fn update_alliance_role(&mut self, role: i16) {
        self.alliance_role = Some(role);
    }

    pub fn add_alliance_contribution(&mut self, bonus_type: AllianceBonusType, points: i64) {
        match bonus_type {
            AllianceBonusType::Recruitment => {
                self.current_alliance_recruitment_contributions += points;
                self.total_alliance_recruitment_contributions += points;
            }
            AllianceBonusType::Metallurgy => {
                self.current_alliance_metallurgy_contributions += points;
                self.total_alliance_metallurgy_contributions += points;
            }
            AllianceBonusType::Philosophy => {
                self.current_alliance_philosophy_contributions += points;
                self.total_alliance_philosophy_contributions += points;
            }
            AllianceBonusType::Commerce => {
                self.current_alliance_commerce_contributions += points;
                self.total_alliance_commerce_contributions += points;
            }
        }
    }

    pub fn get_alliance_contribution(&self, bonus_type: AllianceBonusType) -> i64 {
        match bonus_type {
            AllianceBonusType::Recruitment => self.current_alliance_recruitment_contributions,
            AllianceBonusType::Metallurgy => self.current_alliance_metallurgy_contributions,
            AllianceBonusType::Philosophy => self.current_alliance_philosophy_contributions,
            AllianceBonusType::Commerce => self.current_alliance_commerce_contributions,
        }
    }
}
