use anyhow::Result;
use async_trait::async_trait;

use crate::repository::uow::UnitOfWork;

/// Context which contains JobHandler dependencies.
pub struct JobHandlerContext<'a> {
    pub uow: Box<dyn UnitOfWork<'a> + 'a>,
}

#[async_trait]
pub trait JobHandler: Send + Sync {
    async fn handle<'ctx, 'a>(&'ctx self, ctx: &'ctx JobHandlerContext<'a>) -> Result<()>;
}
