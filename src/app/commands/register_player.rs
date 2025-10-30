use crate::{
    game::models::{Player, Tribe},
    repository::PlayerRepository,
};
use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
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

pub struct RegisterPlayerHandler<'a> {
    repo: Arc<dyn PlayerRepository + 'a>,
}

impl<'a> RegisterPlayerHandler<'a> {
    pub fn new(repo: Arc<dyn PlayerRepository + 'a>) -> Self {
        Self { repo }
    }

    pub async fn handle(&self, command: RegisterPlayer) -> Result<Player> {
        let player = Player {
            id: command.id,
            username: command.username,
            tribe: command.tribe,
        };

        self.repo.create(&player).await?;

        Ok(player)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::models::{Player, Tribe};
    use crate::repository::PlayerRepository;
    use anyhow::Result;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    #[derive(Default)]
    struct MockPlayerRepository {
        created_player: Mutex<Option<Player>>,
    }

    #[async_trait]
    impl PlayerRepository for MockPlayerRepository {
        async fn create(&self, player: &Player) -> Result<()> {
            *self.created_player.lock().unwrap() = Some(player.clone());

            Ok(())
        }

        async fn get_by_id(&self, _player_id: Uuid) -> Result<Option<Player>> {
            Ok(None)
        }
        async fn get_by_username(&self, _username: &str) -> Result<Option<Player>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_register_player_handler() {
        let mock_repo = Arc::new(MockPlayerRepository::default());
        let handler = RegisterPlayerHandler::new(mock_repo.clone());
        let command = RegisterPlayer::new(None, "test_user".to_string(), Tribe::Roman);

        let result = handler.handle(command).await.unwrap();
        let created_player = mock_repo.created_player.lock().unwrap();

        assert_eq!(result.username, "test_user");
        assert_eq!(result.tribe, Tribe::Roman);
        assert_eq!(created_player.as_ref().unwrap().username, "test_user");
    }
}
