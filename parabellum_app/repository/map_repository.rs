use parabellum_game::models::map::{MapField, MapQuadrant, Valley};
use parabellum_types::errors::ApplicationError;

#[async_trait::async_trait]
pub trait MapRepository: Send + Sync {
    async fn find_unoccupied_valley(
        &self,
        quadrant: &MapQuadrant,
    ) -> Result<Valley, ApplicationError>;
    async fn get_field_by_id(&self, id: i32) -> Result<MapField, ApplicationError>;
}
