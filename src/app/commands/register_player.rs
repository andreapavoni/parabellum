use std::sync::Arc;

use anyhow::{Error, Result};

use crate::{
    command::Command,
    game::models::{Player, Tribe},
    repository::Repository,
};

#[derive(Clone)]
pub struct RegisterPlayer {
    username: String,
    tribe: Tribe,
}

impl RegisterPlayer {
    pub fn new(username: String, tribe: Tribe) -> Self {
        Self { username, tribe }
    }
}

#[async_trait::async_trait]
impl Command for RegisterPlayer {
    type Output = Player;

    async fn run(&self, repo: Arc<dyn Repository>) -> Result<Self::Output, Error> {
        repo.register_player(self.username.clone(), self.tribe.clone())
            .await
    }
}
