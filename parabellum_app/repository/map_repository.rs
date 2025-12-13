use parabellum_game::models::map::{MapField, MapQuadrant, Valley};
use parabellum_types::errors::ApplicationError;

#[derive(Debug, Clone)]
pub struct MapRegionTile {
    pub field: MapField,
    pub village_name: Option<String>,
    pub village_population: Option<i32>,
    pub player_name: Option<String>,
    pub tribe: Option<parabellum_types::tribe::Tribe>,
}

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
}
