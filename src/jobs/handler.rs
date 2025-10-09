use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::repository::{ArmyRepository, JobRepository, VillageRepository};

/// Context which contains JobHandler dependencies.
pub struct JobHandlerContext {
    pub job_repo: Arc<dyn JobRepository>,
    pub village_repo: Arc<dyn VillageRepository>,
    pub army_repo: Arc<dyn ArmyRepository>,
    // ...
}

#[async_trait]
pub trait JobHandler: Send + Sync {
    async fn handle(&self, ctx: &JobHandlerContext) -> Result<()>;
}
