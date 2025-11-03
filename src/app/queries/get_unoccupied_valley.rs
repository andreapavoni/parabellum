use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    Result,
    config::Config,
    cqrs::{Query, QueryHandler},
    error::ApplicationError,
    game::models::map::{MapQuadrant, Valley},
    repository::uow::UnitOfWork,
};

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

pub struct GetUnoccupiedValleyHandler {}

impl Default for GetUnoccupiedValleyHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl GetUnoccupiedValleyHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryHandler<GetUnoccupiedValley> for GetUnoccupiedValleyHandler {
    async fn handle(
        &self,
        query: GetUnoccupiedValley,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<<GetUnoccupiedValley as Query>::Output, ApplicationError> {
        let repo = uow.map();
        Ok(repo.find_unoccupied_valley(&query.quadrant).await?)
    }
}

#[cfg(test)]
mod tests {}
