use parabellum_game::models::map::{MapQuadrant, Valley};
use parabellum_types::common::User;

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

pub struct AuthenticateUser {
    pub email: String,
    pub password: String,
}

impl Query for AuthenticateUser {
    type Output = User;
}
