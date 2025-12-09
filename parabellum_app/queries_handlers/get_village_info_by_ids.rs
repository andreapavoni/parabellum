use std::{collections::HashMap, sync::Arc};

use crate::{
    config::Config,
    cqrs::{QueryHandler, queries::GetVillageInfoByIds},
    repository::VillageInfo,
    uow::UnitOfWork,
};
use parabellum_types::{Result, errors::ApplicationError};

pub struct GetVillageInfoByIdsHandler;

impl GetVillageInfoByIdsHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl QueryHandler<GetVillageInfoByIds> for GetVillageInfoByIdsHandler {
    async fn handle(
        &self,
        query: GetVillageInfoByIds,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<HashMap<u32, VillageInfo>, ApplicationError> {
        uow.villages().get_info_by_ids(&query.village_ids).await
    }
}
