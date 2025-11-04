use crate::{
    Result,
    config::Config,
    cqrs::{CommandHandler, commands::RegisterVillage},
    game::models::village::Village,
    repository::{MapRepository, VillageRepository, uow::UnitOfWork},
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

        village_repo.create(&village).await?;

        Ok(())
    }
}
