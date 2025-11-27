use async_trait::async_trait;
use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    config::Config,
    cqrs::{Query, QueryHandler, queries::GetMapRegion},
    uow::UnitOfWork,
};

pub struct GetMapRegionHandler {}

impl GetMapRegionHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryHandler<GetMapRegion> for GetMapRegionHandler {
    async fn handle(
        &self,
        query: GetMapRegion,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<<GetMapRegion as Query>::Output, ApplicationError> {
        let repo = uow.map();
        repo.get_region(query.center.x, query.center.y, query.radius)
            .await
    }
}
