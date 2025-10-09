use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::game::models as game_models;

#[derive(sqlx::Type, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[sqlx(type_name = "tribe", rename_all = "PascalCase")]
pub enum Tribe {
    Roman,
    Gaul,
    Teuton,
    Natar,
    Nature,
}

impl From<Tribe> for game_models::Tribe {
    fn from(db_tribe: Tribe) -> Self {
        match db_tribe {
            Tribe::Roman => game_models::Tribe::Roman,
            Tribe::Gaul => game_models::Tribe::Gaul,
            Tribe::Teuton => game_models::Tribe::Teuton,
            Tribe::Natar => game_models::Tribe::Natar,
            Tribe::Nature => game_models::Tribe::Nature,
        }
    }
}

impl From<game_models::Tribe> for Tribe {
    fn from(game_tribe: game_models::Tribe) -> Self {
        match game_tribe {
            game_models::Tribe::Roman => Tribe::Roman,
            game_models::Tribe::Gaul => Tribe::Gaul,
            game_models::Tribe::Teuton => Tribe::Teuton,
            game_models::Tribe::Natar => Tribe::Natar,
            game_models::Tribe::Nature => Tribe::Nature,
        }
    }
}

#[derive(Debug, FromRow, Clone)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
    pub tribe: Tribe,
}
impl From<Player> for game_models::Player {
    fn from(player: Player) -> Self {
        game_models::Player {
            id: player.id,
            username: player.username,
            tribe: player.tribe.into(),
        }
    }
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
    pub population: i32,
    pub loyalty: i16,
    pub is_capital: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// impl From<Village> for game_models::village::Village {
//     fn from(db_village: Village) -> Self {
//         game_models::village::Village {
//             id: db_village.id as u32,
//             player_id: db_village.player_id,
//             name: db_village.name,
//             position: db_village.position,
//             buildings: db_village.buildings,
//             production: db_village.production,
//             stocks: db_village.stocks,
//             smithy: db_village.smithy_upgrades,
//             population: db_village.population as u32,
//             loyalty: db_village.loyalty as u8,
//             is_capital: db_village.is_capital,
//             updated_at: db_village.updated_at,
//         }
//     }
// }

// impl From<game_models::village::Village> for Village {
//     fn from(game_village: game_models::village::Village) -> Self {
//         Village {
//             id: game_village.id as i32,
//             player_id: game_village.player_id,
//             name: game_village.name,
//             position: game_village.position,
//             buildings: game_village.buildings,
//             production: game_village.production,
//             stocks: game_village.stocks,
//             smithy_upgrades: game_village.smithy.into(),
//             population: game_village.population as i32,
//             loyalty: game_village.loyalty as i16,
//             is_capital: game_village.is_capital,
//             created_at: game_village.updated_at,
//             updated_at: game_village.updated_at,
//         }
//     }
// }

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

impl From<Army> for game_models::army::Army {
    fn from(army: Army) -> Self {
        game_models::army::Army {
            village_id: army.village_id as u32,
            current_map_field_id: Some(army.current_map_field_id as u32),
            player_id: army.player_id,
            units: serde_json::from_value(army.units).unwrap_or_default(),
            smithy: serde_json::from_value(army.smithy).unwrap_or_default(),
            hero: None, // TODO: load hero through join
            tribe: army.tribe.into(),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct MapField {
    pub id: i32,
    pub village_id: Option<i32>,
    pub player_id: Option<Uuid>,
    pub position: serde_json::Value,
    pub topology: serde_json::Value,
}

impl From<MapField> for game_models::map::MapField {
    fn from(map_field: MapField) -> Self {
        game_models::map::MapField {
            id: map_field.id as u32,
            village_id: map_field.village_id.map(|id| id as u32),
            player_id: map_field.player_id,
            position: serde_json::from_value(map_field.position).unwrap(),
            topology: serde_json::from_value(map_field.topology).unwrap(),
        }
    }
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

impl From<Job> for crate::jobs::Job {
    fn from(job: Job) -> Self {
        crate::jobs::Job {
            id: job.id,
            player_id: job.player_id,
            village_id: job.village_id,
            task: serde_json::from_value(job.task).unwrap(),
            status: match job.status {
                JobStatus::Pending => crate::jobs::JobStatus::Pending,
                JobStatus::Processing => crate::jobs::JobStatus::Processing,
                JobStatus::Completed => crate::jobs::JobStatus::Completed,
                JobStatus::Failed => crate::jobs::JobStatus::Failed,
            },
            completed_at: job.completed_at,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}
