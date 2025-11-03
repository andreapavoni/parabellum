use crate::{
    Result,
    config::Config,
    cqrs::{Command, CommandHandler},
    game::models::{Player, map::MapQuadrant, village::Village},
    repository::{MapRepository, VillageRepository, uow::UnitOfWork},
};

use std::sync::Arc;

#[derive(Clone)]
pub struct RegisterVillage {
    pub player: Player,
    pub quadrant: MapQuadrant,
}

impl RegisterVillage {
    pub fn new(player: Player, quadrant: MapQuadrant) -> Self {
        Self { player, quadrant }
    }
}

pub struct RegisterVillageHandler {}

impl Command for RegisterVillage {}

impl Default for RegisterVillageHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterVillageHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<RegisterVillage> for RegisterVillageHandler {
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
