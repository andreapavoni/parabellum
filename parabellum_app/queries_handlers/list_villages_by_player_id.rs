use async_trait::async_trait;
use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    config::Config,
    cqrs::{Query, QueryHandler, queries::ListVillagesByPlayerId},
    uow::UnitOfWork,
};

pub struct ListVillagesByPlayerIdHandler {}

impl ListVillagesByPlayerIdHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryHandler<ListVillagesByPlayerId> for ListVillagesByPlayerIdHandler {
    async fn handle(
        &self,
        query: ListVillagesByPlayerId,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<<ListVillagesByPlayerId as Query>::Output, ApplicationError> {
        let repo = uow.villages();
        repo.list_by_player_id(query.player_id).await
    }
}
