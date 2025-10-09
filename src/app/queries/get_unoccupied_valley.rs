use std::sync::Arc;

use anyhow::{Error, Result};

use crate::{
    game::models::map::{MapQuadrant, Valley},
    query::Query,
    repository::Repository,
};

#[derive(Clone)]
pub struct GetUnoccupiedValley {
    quadrant: Option<MapQuadrant>,
}

impl GetUnoccupiedValley {
    pub fn new(quadrant: Option<MapQuadrant>) -> Self {
        Self { quadrant }
    }
}

#[async_trait::async_trait]
impl Query for GetUnoccupiedValley {
    type Output = Valley;

    async fn run(&self, repo: Arc<dyn Repository>) -> Result<Self::Output, Error> {
        let valley = repo.get_unoccupied_valley(self.quadrant.clone()).await?;

        Ok(valley)
    }
}
