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
