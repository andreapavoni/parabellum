use async_trait::async_trait;
use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    config::Config,
    cqrs::{Query, QueryHandler, queries::GetPlayerByUserId},
    uow::UnitOfWork,
};

pub struct GetPlayerByUserIdHandler {}

impl GetPlayerByUserIdHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryHandler<GetPlayerByUserId> for GetPlayerByUserIdHandler {
    async fn handle(
        &self,
        query: GetPlayerByUserId,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<<GetPlayerByUserId as Query>::Output, ApplicationError> {
        let repo = uow.players();
        repo.get_by_user_id(query.user_id).await
    }
}
