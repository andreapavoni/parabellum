use std::sync::Arc;

use anyhow::{Error, Result};

use crate::{
    command::Command,
    game::models::{village::Village, Player},
    repository::Repository,
};

#[derive(Clone)]
pub struct RegisterVillage {
    player: Player,
}

impl RegisterVillage {
    pub fn new(player: Player) -> Self {
        Self { player }
    }
}

#[async_trait::async_trait]
impl Command for RegisterVillage {
    type Output = Village;

    async fn run(&self, repo: Arc<dyn Repository>) -> Result<Self::Output, Error> {
        let valley = repo.get_unoccupied_valley(None).await?;
        let village = Village::new("New Village".to_string(), &valley, &self.player, true);

        Ok(village)
    }
}
