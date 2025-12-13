use async_trait::async_trait;
use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    config::Config,
    cqrs::{Query, QueryHandler, queries::GetPlayerById},
    uow::UnitOfWork,
};

pub struct GetPlayerByIdHandler {}

impl GetPlayerByIdHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryHandler<GetPlayerById> for GetPlayerByIdHandler {
    async fn handle(
        &self,
        query: GetPlayerById,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<<GetPlayerById as Query>::Output, ApplicationError> {
        let repo = uow.players();
        repo.get_by_id(query.player_id).await
    }
}
