use async_trait::async_trait;
use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    config::Config,
    cqrs::{Query, QueryHandler, queries::GetUserById},
    uow::UnitOfWork,
};

pub struct GetUserByIdHandler {}

impl GetUserByIdHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryHandler<GetUserById> for GetUserByIdHandler {
    async fn handle(
        &self,
        query: GetUserById,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<<GetUserById as Query>::Output, ApplicationError> {
        uow.users().get_by_id(query.id).await
    }
}
