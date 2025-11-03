use std::sync::Arc;

use crate::{
    Result,
    config::Config,
    cqrs::{Command, CommandHandler, Query, QueryHandler},
    error::ApplicationError,
    repository::uow::UnitOfWorkProvider,
};

/// AppBus (Mediator)
/// This struct is the central entry point for all application logic.
/// It does not contain any business logic itself.
/// Its primary roles are:
/// 1. Managing Unit of Work (transaction) lifecycles.
/// 2. Dispatching Commands and Queries to their respective handlers.
pub struct AppBus {
    config: Arc<Config>,
    uow_provider: Arc<dyn UnitOfWorkProvider>,
}

impl AppBus {
    pub fn new(config: Arc<Config>, uow_provider: Arc<dyn UnitOfWorkProvider>) -> Self {
        Self {
            config,
            uow_provider,
        }
    }

    /// Executes a command.
    /// A command is an operation that modifies the system state.
    /// This method manages the transaction:
    /// - It begins a Unit of Work.
    /// - It passes the UoW to the handler.
    /// - If the handler succeeds, it commits the UoW.
    /// - If the handler fails, it rolls back the UoW.
    pub async fn execute<C, H>(&self, cmd: C, handler: H) -> Result<(), ApplicationError>
    where
        C: Command,
        H: CommandHandler<C>,
    {
        let uow = self.uow_provider.begin().await?;

        match handler.handle(cmd, &uow).await {
            Ok(_) => {
                uow.commit().await?; // Commit on success
                Ok(())
            }
            Err(e) => {
                uow.rollback().await?; // Rollback on failure
                Err(e.into())
            }
        }
    }

    /// Executes a query.
    /// A query is an operation that reads system state and returns data.
    /// It should *never* modify the state.
    /// This method ensures the transaction is *always* rolled back.
    pub async fn query<Q, H>(&self, query: Q, handler: H) -> Result<Q::Output, ApplicationError>
    where
        Q: Query,
        H: QueryHandler<Q>,
    {
        let uow = self.uow_provider.begin().await?;

        let result = handler.handle(query, &uow).await;

        // Always rollback a query, as it should never write data.
        uow.rollback().await?;

        Ok(result?)
    }
}
