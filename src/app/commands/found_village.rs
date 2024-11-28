use std::sync::Arc;

use anyhow::{Error, Result};

use crate::{
    command::Command,
    game::models::{village::Village, Player},
    repository::Repository,
};

#[derive(Clone)]
pub struct FoundVillage {
    player: Player,
    is_capital: bool,
}

impl FoundVillage {
    pub fn new(player: Player, is_capital: bool) -> Self {
        Self { player, is_capital }
    }
}

#[async_trait::async_trait]
impl Command for FoundVillage {
    type Output = Village;

    async fn run(&self, repo: Arc<dyn Repository>) -> Result<Self::Output, Error> {
        let valley = repo.get_unoccupied_valley(None).await?;
        let village = Village::new(
            "New Village".to_string(),
            &valley,
            &self.player,
            self.is_capital,
        );

        Ok(village)
    }
}
