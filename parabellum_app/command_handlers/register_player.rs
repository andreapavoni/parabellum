use std::sync::Arc;

use parabellum_game::models::village::Village;
use parabellum_types::Result;
use parabellum_types::common::Player;

use crate::{
    auth::hash_password,
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
        config: &Arc<Config>,
    ) -> Result<()> {
        let player_repo = uow.players();
        let user_repo = uow.users();
        let village_repo = uow.villages();
        let map_repo = uow.map();

        let password_hash = hash_password(&command.password)?;
        user_repo.save(command.email.clone(), password_hash).await?;
        let user = user_repo.get_by_email(&command.email).await?;

        let player = Player {
            id: command.id,
            username: command.username,
            tribe: command.tribe,
            user_id: user.id,
        };
        player_repo.save(&player).await?;

        let valley = map_repo.find_unoccupied_valley(&command.quadrant).await?;

        let village = Village::new(
            "New Village".to_string(),
            &valley,
            &player,
            true,
            config.world_size as i32,
            config.speed,
        );

        village_repo.save(&village).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use parabellum_game::models::map::MapQuadrant;
    use std::sync::Arc;
    use uuid::Uuid;

    use parabellum_types::Result;
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

        let email = "player@example.com".to_string();

        let command = RegisterPlayer {
            id: Uuid::new_v4(),
            username: "TestPlayer".to_string(),
            tribe: Tribe::Roman,
            email: email.clone(),
            password: "some_password".to_string(),
            quadrant: MapQuadrant::NorthEast,
        };

        handler.handle(command.clone(), &mock_uow, &config).await?;

        let saved_player = mock_uow.players().get_by_id(command.id).await?;
        assert_eq!(saved_player.id, command.id);
        assert_eq!(saved_player.username, command.username);
        assert_eq!(saved_player.tribe, command.tribe);

        mock_uow.users().get_by_email(&email).await?;

        let saved_villages = mock_uow.villages().list_by_player_id(command.id).await?;
        let saved_village = saved_villages.first().unwrap();

        assert!(saved_village.position.x > 0);
        assert!(saved_village.position.y > 0);
        assert_eq!(saved_village.tribe, command.tribe);

        Ok(())
    }
}
