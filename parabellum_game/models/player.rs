use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_core::GameError;
use parabellum_types::tribe::Tribe;
pub use parabellum_types::alliance::BonusType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
    pub tribe: Tribe,
    pub user_id: Uuid,
    pub alliance_id: Option<Uuid>,
    pub alliance_role: Option<i32>,
    pub alliance_join_time: Option<chrono::DateTime<chrono::Utc>>,
    pub current_alliance_training_contributions: i64,
    pub current_alliance_armor_contributions: i64,
    pub current_alliance_cp_contributions: i64,
    pub current_alliance_trade_contributions: i64,
    pub total_alliance_training_contributions: i64,
    pub total_alliance_armor_contributions: i64,
    pub total_alliance_cp_contributions: i64,
    pub total_alliance_trade_contributions: i64,
}

impl Player {
    pub fn join_alliance(&mut self, alliance_id: Uuid, role: i32) -> Result<(), GameError> {
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
        self.current_alliance_training_contributions = 0;
        self.current_alliance_armor_contributions = 0;
        self.current_alliance_cp_contributions = 0;
        self.current_alliance_trade_contributions = 0;
        self.total_alliance_training_contributions = 0;
        self.total_alliance_armor_contributions = 0;
        self.total_alliance_cp_contributions = 0;
        self.total_alliance_trade_contributions = 0;
    }

    pub fn update_alliance_role(&mut self, role: i32) {
        self.alliance_role = Some(role);
    }

    pub fn add_alliance_contribution(&mut self, bonus_type: BonusType, points: i64) {
        match bonus_type {
            BonusType::Training => {
                self.current_alliance_training_contributions += points;
                self.total_alliance_training_contributions += points;
            }
            BonusType::Armor => {
                self.current_alliance_armor_contributions += points;
                self.total_alliance_armor_contributions += points;
            }
            BonusType::CombatPoints => {
                self.current_alliance_cp_contributions += points;
                self.total_alliance_cp_contributions += points;
            }
            BonusType::Trade => {
                self.current_alliance_trade_contributions += points;
                self.total_alliance_trade_contributions += points;
            }
        }
    }

    pub fn get_alliance_contribution(&self, bonus_type: BonusType) -> i64 {
        match bonus_type {
            BonusType::Training => self.current_alliance_training_contributions,
            BonusType::Armor => self.current_alliance_armor_contributions,
            BonusType::CombatPoints => self.current_alliance_cp_contributions,
            BonusType::Trade => self.current_alliance_trade_contributions,
        }
    }
}
