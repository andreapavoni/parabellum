use async_trait::async_trait;
use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    config::Config,
    cqrs::{Query, QueryHandler, queries::GetUserByEmail},
    uow::UnitOfWork,
};

pub struct GetUserByEmailHandler {}

impl GetUserByEmailHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryHandler<GetUserByEmail> for GetUserByEmailHandler {
    async fn handle(
        &self,
        query: GetUserByEmail,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<<GetUserByEmail as Query>::Output, ApplicationError> {
        let user_repo = uow.users();
        user_repo.get_by_email(&query.email).await
    }
}
