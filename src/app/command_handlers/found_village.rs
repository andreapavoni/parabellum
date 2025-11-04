use crate::{
    Result,
    config::Config,
    cqrs::{CommandHandler, commands::FoundVillage},
    game::models::{map::Valley, village::Village},
    repository::{MapRepository, VillageRepository, uow::UnitOfWork},
};
use std::sync::Arc;

pub struct FoundVillageCommandHandler {}

impl Default for FoundVillageCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl FoundVillageCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<FoundVillage> for FoundVillageCommandHandler {
    async fn handle(
        &self,
        command: FoundVillage,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<()> {
        let village_id: i32 = command.position.to_id(config.world_size as i32) as i32;
        let village_repo: Arc<dyn VillageRepository + '_> = uow.villages();
        let map_repo: Arc<dyn MapRepository + '_> = uow.map();

        let map_field = map_repo.get_field_by_id(village_id).await?;
        let valley = Valley::try_from(map_field)?;
        let village = Village::new(
            "New Village".to_string(),
            &valley,
            &command.player,
            false,
            config.world_size as i32,
        );

        village_repo.create(&village).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::test_utils::tests::{MockUnitOfWork, assert_handler_success},
        config::Config,
        cqrs::commands::FoundVillage,
        game::{
            models::{Tribe, map::Position},
            test_utils::{PlayerFactoryOptions, player_factory},
        },
        repository::uow::UnitOfWork,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_found_village_handler_success() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let config = Arc::new(Config::from_env());
        let handler = FoundVillageCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        // The mock map repo will return a valley at (10, 10)
        let position = Position { x: 10, y: 10 };
        let command = FoundVillage::new(player.clone(), position);

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
        assert_eq!(village.name, "New Village");
        assert_eq!(village.position, Position { x: 10, y: 10 });
        assert_eq!(village.is_capital, false); // Found village is not capital
    }
}
