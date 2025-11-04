use std::sync::Arc;

use crate::{
    Result,
    config::Config,
    cqrs::{CommandHandler, commands::RegisterPlayer},
    game::models::Player,
    repository::uow::UnitOfWork,
};

pub struct RegisterPlayerCommandHandler {}

impl Default for RegisterPlayerCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterPlayerCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<RegisterPlayer> for RegisterPlayerCommandHandler {
    async fn handle(
        &self,
        command: RegisterPlayer,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
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
