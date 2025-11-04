use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    Result,
    config::Config,
    cqrs::{Query, QueryHandler, queries::GetUnoccupiedValley},
    error::ApplicationError,
    uow::UnitOfWork,
};

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
