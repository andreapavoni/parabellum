use parabellum_game::models::map::{MapField, MapQuadrant, Valley};
use parabellum_types::errors::ApplicationError;

use crate::query_models::MapRegionTile;

#[async_trait::async_trait]
pub trait MapRepository: Send + Sync {
    async fn find_unoccupied_valley(
        &self,
        quadrant: &MapQuadrant,
    ) -> Result<Valley, ApplicationError>;
    async fn get_field_by_id(&self, id: i32) -> Result<MapField, ApplicationError>;
    async fn get_region(
        &self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        world_size: i32,
    ) -> Result<Vec<MapRegionTile>, ApplicationError>;
    async fn get_region_tile_by_field_id(
        &self,
        field_id: i32,
    ) -> Result<Option<MapRegionTile>, ApplicationError>;
    async fn is_unoccupied_valley(&self, field_id: i32) -> Result<bool, ApplicationError>;
}
