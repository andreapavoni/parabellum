use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;
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
