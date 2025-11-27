use async_trait::async_trait;
use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    config::Config,
    cqrs::{Query, QueryHandler, queries::GetVillageById},
    uow::UnitOfWork,
};

pub struct GetVillageByIdHandler {}

impl GetVillageByIdHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryHandler<GetVillageById> for GetVillageByIdHandler {
    async fn handle(
        &self,
        query: GetVillageById,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<<GetVillageById as Query>::Output, ApplicationError> {
        let repo = uow.villages();
        repo.get_by_id(query.id).await
    }
}
