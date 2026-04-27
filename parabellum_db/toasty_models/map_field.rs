use uuid::Uuid;

use crate::models as db_models;

#[derive(Debug, Clone, toasty::Model)]
#[table = "map_fields"]
pub struct MapFieldDbRow {
    #[key]
    pub id: i32,
    #[index]
    pub village_id: Option<i32>,
    pub player_id: Option<Uuid>,

    #[serialize(json)]
    pub position: serde_json::Value,

    #[serialize(json)]
    pub topology: serde_json::Value,
}

impl From<MapFieldDbRow> for db_models::MapField {
    fn from(field: MapFieldDbRow) -> Self {
        Self {
            id: field.id,
            village_id: field.village_id,
            player_id: field.player_id,
            position: field.position,
            topology: field.topology,
        }
    }
}
