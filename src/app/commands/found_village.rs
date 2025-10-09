use std::sync::Arc;

use anyhow::{Error, Result};

use crate::{
    command::Command,
    game::models::{map::Position, village::Village, Player},
    repository::Repository,
};

#[derive(Clone)]
pub struct FoundVillage {
    player: Player,
    position: Position,
}

impl FoundVillage {
    pub fn new(player: Player, position: Position) -> Self {
        Self { player, position }
    }
}

#[async_trait::async_trait]
impl Command for FoundVillage {
    type Output = Village;

    async fn run(&self, repo: Arc<dyn Repository>) -> Result<Self::Output, Error> {
        // TODO: get world size from some global config
        let world_size = 100;
        let valley = repo
            .get_valley_by_id(self.position.to_id(world_size))
            .await?;

        let village = Village::new("New Village".to_string(), &valley, &self.player, false);

        Ok(village)
    }
}
