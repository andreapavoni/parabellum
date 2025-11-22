use async_trait::async_trait;
use std::sync::Arc;

use parabellum_types::errors::{AppError, ApplicationError, DbError};

use crate::{
    auth::verify_password,
    config::Config,
    cqrs::{Query, QueryHandler, queries::AuthenticateUser},
    uow::UnitOfWork,
};

pub struct AuthenticateUserHandler {}

impl AuthenticateUserHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryHandler<AuthenticateUser> for AuthenticateUserHandler {
    async fn handle(
        &self,
        query: AuthenticateUser,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<<AuthenticateUser as Query>::Output, ApplicationError> {
        let user_repo = uow.users();
        if let Ok(user) = user_repo.get_by_email(&query.email).await {
            if verify_password(&user.password_hash(), &query.password).is_ok() {
                return Ok(user);
            }
            return Err(ApplicationError::App(AppError::WrongAuthCredentials));
        }
        Err(ApplicationError::Db(DbError::UserByEmailNotFound(
            query.email,
        )))
    }
}
