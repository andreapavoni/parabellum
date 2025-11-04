use crate::{
    Result,
    config::Config,
    cqrs::{CommandHandler, commands::RegisterVillage},
    game::models::village::Village,
    repository::{MapRepository, VillageRepository},
    uow::UnitOfWork,
};

use std::sync::Arc;

pub struct RegisterVillageCommandHandler {}

impl Default for RegisterVillageCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterVillageCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<RegisterVillage> for RegisterVillageCommandHandler {
    async fn handle(
        &self,
        cmd: RegisterVillage,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<()> {
        let map_repo: Arc<dyn MapRepository + '_> = uow.map();
        let valley = map_repo.find_unoccupied_valley(&cmd.quadrant).await?;

        let village_repo: Arc<dyn VillageRepository + '_> = uow.villages();
        let village = Village::new(
            "New Village".to_string(),
            &valley,
            &cmd.player,
            true,
            config.world_size as i32,
        );

        village_repo.save(&village).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::test_utils::tests::{MockUnitOfWork, assert_handler_success},
        config::Config,
        cqrs::commands::RegisterVillage,
        game::{
            models::{
                Tribe,
                map::{MapQuadrant, Position},
            },
            test_utils::{PlayerFactoryOptions, player_factory},
        },
        uow::UnitOfWork,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_register_village_handler_success() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let config = Arc::new(Config::from_env());
        let handler = RegisterVillageCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Teuton),
            ..Default::default()
        });

        mock_uow.players().save(&player).await.unwrap();

        let command = RegisterVillage::new(player.clone(), MapQuadrant::NorthEast);

        let result = handler.handle(command, &mock_uow, &config).await;
        assert_handler_success(result);

        let villages = mock_uow
            .villages()
            .list_by_player_id(player.id)
            .await
            .unwrap();

        assert_eq!(villages.len(), 1, "One village should be created");

        let village = &villages[0];
        assert_eq!(village.player_id, player.id);
        assert_eq!(village.is_capital, true); // First village is capital

        // MockMapRepository (from test_utils) returns a default valley at (10, 10)
        assert_eq!(village.position, Position { x: 10, y: 10 });
    }
}
