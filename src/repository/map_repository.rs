use crate::game::models::map::{MapField, MapQuadrant, Valley};
use anyhow::Result;

#[async_trait::async_trait]
pub trait MapRepository: Send + Sync {
    async fn find_unoccupied_valley(&self, quadrant: &MapQuadrant) -> Result<Valley>;
    async fn get_field_by_id(&self, id: i32) -> Result<Option<MapField>>;
    // Potrebbe avere anche metodi per popolare la mappa all'inizio
    // async fn bulk_create_fields(&self, fields: Vec<NewMapField>) -> Result<()>;
}
