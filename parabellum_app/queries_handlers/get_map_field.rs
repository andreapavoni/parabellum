use async_trait::async_trait;
use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    config::Config,
    cqrs::{Query, QueryHandler, queries::GetMapField},
    uow::UnitOfWork,
};

pub struct GetMapFieldHandler {}

impl GetMapFieldHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryHandler<GetMapField> for GetMapFieldHandler {
    async fn handle(
        &self,
        query: GetMapField,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<<GetMapField as Query>::Output, ApplicationError> {
        let repo = uow.map();
        repo.get_field_by_id(query.field_id as i32).await
    }
}
