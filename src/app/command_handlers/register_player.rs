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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::test_utils::tests::{MockUnitOfWork, assert_handler_success},
        config::Config,
        cqrs::commands::RegisterPlayer,
        game::models::Tribe,
        repository::uow::UnitOfWork,
    };
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_register_player_handler_success() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let config = Arc::new(Config::from_env());
        let handler = RegisterPlayerCommandHandler::new();

        let command = RegisterPlayer {
            id: Uuid::new_v4(),
            username: "TestPlayer".to_string(),
            tribe: Tribe::Roman,
        };

        let result = handler.handle(command.clone(), &mock_uow, &config).await;
        assert_handler_success(result);

        let saved_player_result = mock_uow.players().get_by_id(command.id).await;
        assert!(
            saved_player_result.is_ok(),
            "Player should be found in the repository"
        );

        let saved_player = saved_player_result.unwrap();
        assert_eq!(saved_player.id, command.id);
        assert_eq!(saved_player.username, command.username);
        assert_eq!(saved_player.tribe, command.tribe);
    }
}
