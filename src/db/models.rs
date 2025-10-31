use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(sqlx::Type, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[sqlx(type_name = "tribe", rename_all = "PascalCase")]
pub enum Tribe {
    Roman,
    Gaul,
    Teuton,
    Natar,
    Nature,
}

#[derive(Debug, FromRow, Clone)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
    pub tribe: Tribe,
}

#[derive(Debug, FromRow)]
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

#[derive(Debug, FromRow)]
pub struct Army {
    pub id: Uuid,
    pub village_id: i32,
    pub current_map_field_id: i32,
    pub hero_id: Option<Uuid>,
    pub units: serde_json::Value,
    pub smithy: serde_json::Value,
    pub tribe: Tribe,
    pub player_id: Uuid,
}

#[derive(Debug, FromRow)]
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
