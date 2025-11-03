use crate::{
    Result,
    error::ApplicationError,
    game::models::map::{MapField, MapQuadrant, Valley},
};

#[async_trait::async_trait]
pub trait MapRepository: Send + Sync {
    async fn find_unoccupied_valley(
        &self,
        quadrant: &MapQuadrant,
    ) -> Result<Valley, ApplicationError>;
    async fn get_field_by_id(&self, id: i32) -> Result<MapField, ApplicationError>;
    // async fn bulk_create_fields(&self, fields: Vec<NewMapField>) -> Result<()>;
}
