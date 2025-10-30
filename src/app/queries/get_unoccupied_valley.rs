use anyhow::Result;
use async_trait::async_trait;

use crate::{
    cqrs::{Query, QueryHandler},
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

impl GetUnoccupiedValleyHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn handle(
        &self,
        query: GetUnoccupiedValley,
        uow: &Box<dyn UnitOfWork<'_>>,
    ) -> Result<Valley> {
        let repo = uow.map();
        repo.find_unoccupied_valley(&query.quadrant).await
    }
}

#[async_trait]
impl QueryHandler<GetUnoccupiedValley> for GetUnoccupiedValleyHandler {
    async fn handle(
        &self,
        query: GetUnoccupiedValley,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
    ) -> Result<<GetUnoccupiedValley as Query>::Output> {
        let repo = uow.map();
        repo.find_unoccupied_valley(&query.quadrant).await
    }
}

#[cfg(test)]
mod tests {}
