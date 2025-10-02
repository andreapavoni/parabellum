use crate::{
    game::models::{Player, Tribe},
    repository::PlayerRepository,
};
use anyhow::Result;
use std::sync::Arc;

#[derive(Clone)]
pub struct RegisterPlayer {
    pub username: String,
    pub tribe: Tribe,
}

impl RegisterPlayer {
    pub fn new(username: String, tribe: Tribe) -> Self {
        Self { username, tribe }
    }
}

pub struct RegisterPlayerHandler {
    repo: Arc<dyn PlayerRepository>,
}

impl RegisterPlayerHandler {
    pub fn new(repo: Arc<dyn PlayerRepository>) -> Self {
        Self { repo }
    }

    pub async fn handle(&self, command: RegisterPlayer) -> Result<Player> {
        self.repo.create(command.username, command.tribe).await
    }
}
