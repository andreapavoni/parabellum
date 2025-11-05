pub mod commands;
pub mod queries;

use async_trait::async_trait;
use parabellum_core::ApplicationError;
use std::sync::Arc;

use crate::{config::Config, uow::UnitOfWork};

/// A marker trait for Command structs.
/// Commands are operations that change the state of the system.
pub trait Command: Send + Sync {}

/// A trait for handlers that execute Commands.
/// It receives the command and a Unit of Work (&Box<dyn UnitOfWork...>) to use.
/// It should NOT manage the transaction lifecycle (commit/rollback);
/// that is the job of the AppBus.
#[async_trait]
pub trait CommandHandler<C: Command> {
    async fn handle(
        &self,
        cmd: C,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError>;
}

/// A marker trait for Query structs.
/// Queries are operations that read the state of the system.
pub trait Query: Send + Sync {
    /// The data type that this query will return.
    type Output: Send + Sync;
}

/// A trait for handlers that execute Queries.
/// It receives the query and a Unit of Work to read data.
#[async_trait]
pub trait QueryHandler<Q: Query> {
    async fn handle(
        &self,
        query: Q,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<Q::Output, ApplicationError>;
}
