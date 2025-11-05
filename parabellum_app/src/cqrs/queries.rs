use parabellum_game::models::map::{MapQuadrant, Valley};

use crate::cqrs::Query;

#[derive(Debug, Clone)]
pub struct GetUnoccupiedValley {
    pub quadrant: MapQuadrant,
}

impl GetUnoccupiedValley {
    pub fn new(quadrant: Option<MapQuadrant>) -> Self {
        Self {
            quadrant: quadrant.unwrap_or(MapQuadrant::NorthEast),
        }
    }
}

impl Query for GetUnoccupiedValley {
    type Output = Valley;
}
