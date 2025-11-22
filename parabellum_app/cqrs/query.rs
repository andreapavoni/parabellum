use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;
use std::sync::Arc;

use crate::{config::Config, uow::UnitOfWork};

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
