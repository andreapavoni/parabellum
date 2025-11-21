use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(sqlx::Type, Debug, Clone, Copy, Serialize, Deserialize)]
#[sqlx(type_name = "tribe", rename_all = "PascalCase")]
pub enum Tribe {
    Roman,
    Gaul,
    Teuton,
    Natar,
    Nature,
}

impl From<String> for Tribe {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Roman" => Tribe::Roman,
            "Gaul" => Tribe::Gaul,
            "Teuton" => Tribe::Teuton,
            "Natar" => Tribe::Natar,
            "Nature" => Tribe::Nature,
            _ => Tribe::Roman, // Default fallback
        }
    }
}

#[derive(Debug, FromRow, Clone)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
    pub tribe: Tribe,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub alliance_id: Option<Uuid>,
    pub alliance_role: Option<i32>,
    pub alliance_join_time: Option<DateTime<Utc>>,
    pub current_alliance_training_contributions: Option<i64>,
    pub current_alliance_armor_contributions: Option<i64>,
    pub current_alliance_cp_contributions: Option<i64>,
    pub current_alliance_trade_contributions: Option<i64>,
    pub total_alliance_training_contributions: Option<i64>,
    pub total_alliance_armor_contributions: Option<i64>,
    pub total_alliance_cp_contributions: Option<i64>,
    pub total_alliance_trade_contributions: Option<i64>,
}

#[derive(Debug, FromRow, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Village {
    pub id: i32,
    pub player_id: Uuid,
    pub name: String,
    pub position: serde_json::Value,
    pub buildings: serde_json::Value,
    pub production: serde_json::Value,
    pub stocks: serde_json::Value,
    pub smithy_upgrades: serde_json::Value,
    pub academy_research: serde_json::Value,
    pub population: i32,
    pub loyalty: i16,
    pub is_capital: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Army {
    pub id: Uuid,
    pub village_id: i32,
    pub player_id: Uuid,
    pub current_map_field_id: Option<i32>,
    pub tribe: Tribe,
    pub units: serde_json::Value,
    pub smithy: serde_json::Value,
    pub hero_id: Option<Uuid>,
    pub hero_level: Option<i16>,
    pub hero_resource_focus: Option<serde_json::Value>,
    pub hero_health: Option<i16>,
    pub hero_experience: Option<i32>,
    pub hero_strength_points: Option<i16>,
    pub hero_off_bonus_points: Option<i16>,
    pub hero_def_bonus_points: Option<i16>,
    pub hero_resources_points: Option<i16>,
    pub hero_regeneration_points: Option<i16>,
    pub hero_unassigned_points: Option<i16>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MapField {
    pub id: i32,
    pub village_id: Option<i32>,
    pub player_id: Option<Uuid>,
    pub position: serde_json::Value,
    pub topology: serde_json::Value,
}

#[derive(sqlx::Type, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[sqlx(type_name = "job_status", rename_all = "PascalCase")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(FromRow, Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub player_id: Uuid,
    pub village_id: i32,
    pub task: serde_json::Value,
    pub status: JobStatus,
    pub completed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Hero {
    pub id: Uuid,
    pub player_id: Uuid,
    pub village_id: i32,
    pub tribe: Tribe,
    pub level: i16,
    pub health: i16,
    pub experience: i32,
    pub resource_focus: serde_json::Value,
    pub strength_points: i32,
    pub off_bonus_points: i16,
    pub def_bonus_points: i16,
    pub regeneration_points: i16,
    pub resources_points: i16,
    pub unassigned_points: i16,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketplaceOffer {
    pub id: Uuid,
    pub player_id: Uuid,
    pub village_id: i32,
    pub offer_resources: serde_json::Value,
    pub seek_resources: serde_json::Value,
    pub merchants_required: i16,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MapFlag {
    pub id: Uuid,
    pub alliance_id: Option<Uuid>,
    pub player_id: Option<Uuid>,
    pub target_id: Option<Uuid>,
    pub position: Option<serde_json::Value>,
    pub flag_type: i16,
    pub color: i16,
    pub text: Option<String>,
    pub created_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
