use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

mod conversions;

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
    pub user_id: Uuid,
    pub culture_points: i32,
}

#[derive(Debug, FromRow, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct MapField {
    pub id: i32,
    pub village_id: Option<i32>,
    pub player_id: Option<Uuid>,
    pub position: serde_json::Value,
    pub topology: serde_json::Value,
}
