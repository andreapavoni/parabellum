use std::sync::Arc;

use anyhow::{Error, Result};

use super::Command;
use crate::{
    game::models::{Player, Tribe},
    repository::Repository,
};

#[derive(Clone)]
pub struct RegisterPlayerCommand {
    username: String,
    tribe: Tribe,
}

impl RegisterPlayerCommand {
    pub fn new(username: String, tribe: Tribe) -> Self {
        Self { username, tribe }
    }
}

#[async_trait::async_trait]
impl Command for RegisterPlayerCommand {
    type Output = Player;

    async fn run(&self, repo: Arc<dyn Repository>) -> Result<Self::Output, Error> {
        repo.register_player(self.username.clone(), self.tribe.clone())
            .await
    }
}
