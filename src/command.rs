use std::sync::Arc;

use anyhow::{Error, Result};

use crate::repository::Repository;

#[async_trait::async_trait]
pub trait Command {
    type Output;

    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn run(&self, repo: Arc<dyn Repository>) -> Result<Self::Output, Error>;
}
