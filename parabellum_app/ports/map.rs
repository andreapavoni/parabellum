use parabellum_game::models::map::{MapField, MapQuadrant, Valley};
use parabellum_types::errors::ApplicationError;

use crate::read_models::MapRegionTile;

#[async_trait::async_trait]
/// Read-only map port used by application services and HTTP queries.
pub trait MapRepository: Send + Sync {
    /// Finds an unoccupied valley in the requested quadrant.
    async fn find_unoccupied_valley(
        &self,
        quadrant: &MapQuadrant,
    ) -> Result<Valley, ApplicationError>;
    /// Returns a single map field by deterministic id.
    async fn get_field_by_id(&self, id: i32) -> Result<MapField, ApplicationError>;
    /// Returns a map region centered on `(center_x, center_y)` with the given radius.
    async fn get_region(
        &self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        world_size: i32,
    ) -> Result<Vec<MapRegionTile>, ApplicationError>;
    /// Returns the region tile payload for one field if present.
    async fn get_region_tile_by_field_id(
        &self,
        field_id: i32,
    ) -> Result<Option<MapRegionTile>, ApplicationError>;
    /// Checks whether a field is currently an unoccupied valley.
    async fn is_unoccupied_valley(&self, field_id: i32) -> Result<bool, ApplicationError>;
}
