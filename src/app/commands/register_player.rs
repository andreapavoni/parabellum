use uuid::Uuid;

use crate::{
    Result,
    cqrs::{Command, CommandHandler},
    game::models::{Player, Tribe},
    repository::uow::UnitOfWork,
};

#[derive(Debug, Clone)]
pub struct RegisterPlayer {
    pub id: Uuid,
    pub username: String,
    pub tribe: Tribe,
}

impl RegisterPlayer {
    pub fn new(id: Option<Uuid>, username: String, tribe: Tribe) -> Self {
        Self {
            id: id.unwrap_or(Uuid::new_v4()),
            username,
            tribe,
        }
    }
}

impl Command for RegisterPlayer {}

pub struct RegisterPlayerHandler {}

impl Default for RegisterPlayerHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterPlayerHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<RegisterPlayer> for RegisterPlayerHandler {
    async fn handle(
        &self,
        command: RegisterPlayer,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
    ) -> Result<()> {
        let player = Player {
            id: command.id,
            username: command.username,
            tribe: command.tribe,
        };

        let repo = uow.players();
        repo.create(&player).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {}
