//! Map request models.

/// Request to load a square map region.
#[derive(Debug, Clone, Copy)]
pub struct GetMapRegionRequest {
    /// Center x coordinate.
    pub center_x: i32,
    /// Center y coordinate.
    pub center_y: i32,
    /// Region radius around the center tile.
    pub radius: i32,
    /// World size used for coordinate wrapping.
    pub world_size: i32,
}

/// Request to load a single map field.
#[derive(Debug, Clone, Copy)]
pub struct GetMapFieldRequest {
    /// Field id to load.
    pub field_id: u32,
}

/// Request to load the region tile representation for one field id.
#[derive(Debug, Clone, Copy)]
pub struct GetMapRegionTileByFieldIdRequest {
    /// Field id to load.
    pub field_id: u32,
}
