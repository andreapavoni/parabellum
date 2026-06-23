//! Map application use cases.
//!
//! This service owns app-facing map reads and delegates persistence/query
//! access to `MapReadPort`.

use std::sync::Arc;

use parabellum_game::models::map::MapField;
use parabellum_types::errors::ApplicationError;

use crate::{
    map::{
        ports::MapReadPort,
        requests::{GetMapFieldRequest, GetMapRegionRequest, GetMapRegionTileByFieldIdRequest},
    },
    read_models::MapRegionTile,
};

/// Application service for map reads.
#[derive(Clone)]
pub struct MapUseCases {
    map: Arc<dyn MapReadPort>,
}

impl MapUseCases {
    /// Creates map use cases from the map read port.
    pub fn new(map: Arc<dyn MapReadPort>) -> Self {
        Self { map }
    }

    /// Loads a map region centered on the requested coordinates.
    pub async fn get_map_region(
        &self,
        request: GetMapRegionRequest,
    ) -> Result<Vec<MapRegionTile>, ApplicationError> {
        self.map
            .get_region(
                request.center_x,
                request.center_y,
                request.radius,
                request.world_size,
            )
            .await
    }

    /// Loads one map field by id.
    pub async fn get_map_field(
        &self,
        request: GetMapFieldRequest,
    ) -> Result<MapField, ApplicationError> {
        self.map.get_field_by_id(request.field_id as i32).await
    }

    /// Loads one map region tile by field id.
    pub async fn get_map_region_tile_by_field_id(
        &self,
        request: GetMapRegionTileByFieldIdRequest,
    ) -> Result<Option<MapRegionTile>, ApplicationError> {
        self.map
            .get_region_tile_by_field_id(request.field_id as i32)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use parabellum_game::models::map::{MapField, MapFieldTopology};
    use parabellum_types::map::{Position, ValleyTopology};

    use crate::{
        map::{
            GetMapFieldRequest, GetMapRegionRequest, GetMapRegionTileByFieldIdRequest, MapUseCases,
        },
        read_models::MapRegionTile,
        test_utils::tests::MockMapReadPort,
    };

    fn field(id: u32) -> MapField {
        MapField {
            id,
            position: Position { x: 1, y: 1 },
            village_id: None,
            topology: MapFieldTopology::Valley(ValleyTopology(4, 4, 4, 6)),
            player_id: None,
        }
    }

    #[tokio::test]
    async fn map_reads_delegate_to_map_read_port() {
        let world_size = 100;
        let field_id = Position { x: 1, y: 1 }.to_id(world_size);
        let port = MockMapReadPort::with_fields(HashMap::from([(field_id, field(field_id))]));
        let use_cases = MapUseCases::new(Arc::new(port));

        let region = use_cases
            .get_map_region(GetMapRegionRequest {
                center_x: 1,
                center_y: 1,
                radius: 0,
                world_size,
            })
            .await
            .unwrap();
        let field = use_cases
            .get_map_field(GetMapFieldRequest { field_id })
            .await
            .unwrap();
        let tile = use_cases
            .get_map_region_tile_by_field_id(GetMapRegionTileByFieldIdRequest { field_id })
            .await
            .unwrap();

        assert_eq!(region.len(), 1);
        assert_eq!(region[0].field.id, field_id);
        assert_eq!(field.id, field_id);
        assert!(matches!(tile, Some(MapRegionTile { .. })));
    }
}
