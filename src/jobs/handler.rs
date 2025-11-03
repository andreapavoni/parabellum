use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::{
    Result, config::Config, error::ApplicationError, jobs::Job, repository::uow::UnitOfWork,
};

/// Context which contains JobHandler dependencies.
pub struct JobHandlerContext<'a> {
    pub uow: Box<dyn UnitOfWork<'a> + 'a>,
    pub config: Arc<Config>,
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
