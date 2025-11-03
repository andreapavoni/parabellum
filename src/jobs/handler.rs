use async_trait::async_trait;
use serde_json::Value;

use crate::{Result, error::ApplicationError, jobs::Job, repository::uow::UnitOfWork};

/// Context which contains JobHandler dependencies.
pub struct JobHandlerContext<'a> {
    pub uow: Box<dyn UnitOfWork<'a> + 'a>,
}

#[async_trait]
pub trait JobHandler: Send + Sync {
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<()>;
}

/// Defines a registry that knows how to build a JobHandler
/// from a task_type string and JSON data.
#[async_trait]
pub trait JobRegistry: Send + Sync {
    /// Returns the correct Box<dyn JobHandler> for a given task.
    fn get_handler(
        &self,
        task_type: &str,
        data: &Value,
    ) -> Result<Box<dyn JobHandler>, ApplicationError>;
}
