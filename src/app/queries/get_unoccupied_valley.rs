use std::sync::Arc;

use anyhow::Result;

use crate::{
    game::models::map::{MapQuadrant, Valley},
    repository::MapRepository,
};

#[derive(Clone)]
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

pub struct GetUnoccupiedValleyHandler<'a> {
    repo: Arc<dyn MapRepository + 'a>,
}

impl<'a> GetUnoccupiedValleyHandler<'a> {
    pub fn new(repo: Arc<dyn MapRepository + 'a>) -> Self {
        Self { repo }
    }

    pub async fn handle(&self, query: GetUnoccupiedValley) -> Result<Valley> {
        self.repo.find_unoccupied_valley(&query.quadrant).await
    }
}
