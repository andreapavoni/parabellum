use std::sync::Arc;

use anyhow::{Error, Result};
use mini_cqrs_es::{Aggregate, Command, Event};

use crate::{
    app::events::GameEvent,
    game::models::{village::Village, Tribe},
    repository::Repository,
};

pub struct RegisterPlayerCommand {
    repo: Arc<dyn Repository>,
    username: String,
    tribe: Tribe,
}

impl RegisterPlayerCommand {
    pub fn new(repo: Arc<dyn Repository>, username: String, tribe: Tribe) -> Self {
        Self {
            repo: repo.clone(),
            username,
            tribe,
        }
    }
}

#[async_trait::async_trait]
impl Command for RegisterPlayerCommand {
    type Aggregate = MyAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<Event>, Error> {
        let player = self
            .repo
            .register_player(self.username.clone(), self.tribe.clone())
            .await?;
        let valley = self.repo.get_unoccupied_valley(None).await?;
        let village = Village::new("New village".to_string(), &valley, &player, true);

        println!("{}", serde_json::json!(village.clone()));

        Ok(vec![
            GameEvent::PlayerRegistered(player),
            GameEvent::VillageFounded(village),
        ])
    }
}
