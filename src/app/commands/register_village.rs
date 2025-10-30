use crate::{
    cqrs::{Command, CommandHandler},
    game::models::{map::MapQuadrant, village::Village, Player},
    repository::{uow::UnitOfWork, MapRepository, VillageRepository},
};

use anyhow::Result;
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

impl RegisterVillageHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<RegisterVillage> for RegisterVillageHandler {
    async fn handle(&self, cmd: RegisterVillage, uow: &Box<dyn UnitOfWork<'_> + '_>) -> Result<()> {
        let map_repo: Arc<dyn MapRepository + '_> = uow.map();
        let valley = map_repo.find_unoccupied_valley(&cmd.quadrant).await?;

        let village_repo: Arc<dyn VillageRepository + '_> = uow.villages();
        let village = Village::new("New Village".to_string(), &valley, &cmd.player, true);

        village_repo.create(&village).await?;

        Ok(())
    }
}
