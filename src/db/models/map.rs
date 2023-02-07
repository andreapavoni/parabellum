use anyhow::Error;
use ormlite::model::*;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;

use crate::game::models::map::{
    MapField as GameMapField, MapFieldTopology, Oasis, Position, Valley,
};

#[derive(Model, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[ormlite(table = "map_fields")]
pub struct MapField {
    #[ormlite(primary_key)]
    pub id: u32,
    pub player_id: Option<Uuid>,
    pub village_id: Option<u32>,
    pub position: Json<Position>,
    pub topology: Json<MapFieldTopology>,
}

impl From<MapField> for GameMapField {
    fn from(f: MapField) -> Self {
        Self {
            id: f.id,
            player_id: f.player_id,
            village_id: f.village_id,
            position: f.position.as_ref().clone(),
            topology: f.topology.as_ref().clone(),
        }
    }
}

impl TryFrom<MapField> for Valley {
    type Error = Error;

    fn try_from(f: MapField) -> Result<Self, Self::Error> {
        match f.topology.as_ref().clone() {
            MapFieldTopology::Valley(topology) => Ok(Self {
                id: f.id,
                player_id: f.player_id,
                village_id: f.village_id,
                position: f.position.as_ref().clone(),
                topology,
            }),
            _ => Err(Error::msg("This map field is not a Valley")),
        }
    }
}

impl TryFrom<MapField> for Oasis {
    type Error = Error;

    fn try_from(value: MapField) -> Result<Self, Self::Error> {
        match value.topology.as_ref().clone() {
            MapFieldTopology::Oasis(topology) => Ok(Self {
                id: value.id,
                player_id: value.player_id,
                village_id: value.village_id,
                position: value.position.as_ref().clone(),
                topology,
            }),
            _ => Err(Error::msg("This map field is not an Oasis")),
        }
    }
}

impl From<GameMapField> for MapField {
    fn from(f: GameMapField) -> Self {
        Self {
            id: f.id,
            player_id: f.player_id,
            village_id: f.village_id,
            position: Json(f.position),
            topology: Json(f.topology),
        }
    }
}

impl From<Valley> for MapField {
    fn from(valley: Valley) -> Self {
        MapField {
            id: valley.id,
            player_id: valley.player_id,
            village_id: valley.village_id,
            position: Json(valley.position),
            topology: Json(MapFieldTopology::Valley(valley.topology)),
        }
    }
}

impl From<Oasis> for MapField {
    fn from(oasis: Oasis) -> Self {
        MapField {
            id: oasis.id,
            player_id: oasis.player_id,
            village_id: oasis.village_id,
            position: Json(oasis.position),
            topology: Json(MapFieldTopology::Oasis(oasis.topology)),
        }
    }
}
