pub mod commands;
pub mod jobs;

use anyhow::{Error, Result};
use std::sync::Arc;

use crate::{command::Command, query::Query, repository::Repository};

// RegisterPlayer
// Attack
// Raid
// Reinforce
// ReturnArmy
// SendMerchant
// ReturnMerchant
// TrainBarracksUnit
// TrainStableUnit
// TrainWorkshopUnit
// TrainExpansionUnit
// TrainTrapperUnit
// TrainGreatBarracksUnit
// TrainGreatStableUnit
// TrainGreatWorkshopUnit
// ResearchAcademy
// ResearchSmithy
// StartTownHallCelebration
// StartBreweryCelebration

pub struct App {
    repo: Arc<dyn Repository>,
}

impl App {
    pub fn new(repo: Arc<dyn Repository>) -> Self {
        Self { repo }
    }

    pub async fn command<C>(&self, cmd: C) -> Result<C::Output, Error>
    where
        C: Command + Send + Sync,
    {
        cmd.validate()?;
        cmd.run(self.repo.clone()).await
    }

    pub async fn query<Q>(&self, query: Q) -> Result<Q::Output, Error>
    where
        Q: Query + Send + Sync,
    {
        query.validate()?;
        query.run(self.repo.clone()).await
    }
}
