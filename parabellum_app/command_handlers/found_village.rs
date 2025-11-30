use std::sync::Arc;

use parabellum_game::models::{map::Valley, village::Village};
use parabellum_types::Result;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::FoundVillage},
    repository::{MapRepository, VillageRepository},
    uow::UnitOfWork,
};

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
            config.speed,
        );

        village_repo.save(&village).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parabellum_game::test_utils::{PlayerFactoryOptions, player_factory};
    use parabellum_types::Result;
    use parabellum_types::{map::Position, tribe::Tribe};

    use super::*;
    use crate::{
        config::Config, cqrs::commands::FoundVillage, test_utils::tests::MockUnitOfWork,
        uow::UnitOfWork,
    };

    #[tokio::test]
    async fn test_found_village_handler_success() -> Result<()> {
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

        handler.handle(command, &mock_uow, &config).await?;

        let villages = mock_uow.villages().list_by_player_id(player.id).await?;
        assert_eq!(villages.len(), 1, "One village should be created");

        let village = &villages[0];
        assert_eq!(village.player_id, player.id);
        assert_eq!(village.name, "New Village");
        assert_eq!(village.position, Position { x: 10, y: 10 });
        assert!(!village.is_capital); // Found village is not capital
        Ok(())
    }
}
