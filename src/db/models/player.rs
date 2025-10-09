use ormlite::model::*;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;

use crate::game::models::Tribe;

#[derive(Model, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[ormlite(table = "players")]
pub struct Player {
    #[ormlite(primary_key)]
    pub id: Uuid,
    pub username: String,
    pub tribe: Json<Tribe>,
}

impl From<Player> for crate::game::models::Player {
    fn from(f: Player) -> Self {
        Self {
            id: f.id,
            username: f.username,
            tribe: f.tribe.as_ref().clone(),
        }
    }
}
