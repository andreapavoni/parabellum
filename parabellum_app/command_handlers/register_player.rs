use std::sync::Arc;

use parabellum_core::Result;
use parabellum_types::common::Player;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::RegisterPlayer},
    uow::UnitOfWork,
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
        repo.save(&player).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use parabellum_core::Result;
    use parabellum_types::tribe::Tribe;

    use super::*;
    use crate::{
        config::Config, cqrs::commands::RegisterPlayer, test_utils::tests::MockUnitOfWork,
        uow::UnitOfWork,
    };

    #[tokio::test]
    async fn test_register_player_handler_success() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let config = Arc::new(Config::from_env());
        let handler = RegisterPlayerCommandHandler::new();

        let command = RegisterPlayer {
            id: Uuid::new_v4(),
            username: "TestPlayer".to_string(),
            tribe: Tribe::Roman,
        };

        handler.handle(command.clone(), &mock_uow, &config).await?;

        let saved_player = mock_uow.players().get_by_id(command.id).await?;
        assert_eq!(saved_player.id, command.id);
        assert_eq!(saved_player.username, command.username);
        assert_eq!(saved_player.tribe, command.tribe);
        Ok(())
    }
}
