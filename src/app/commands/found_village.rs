use crate::{
    Result,
    config::Config,
    cqrs::{Command, CommandHandler},
    game::models::{
        Player,
        map::{Position, Valley},
        village::Village,
    },
    repository::{MapRepository, VillageRepository, uow::UnitOfWork},
};
use std::sync::Arc;

#[derive(Clone)]
pub struct FoundVillage {
    pub player: Player,
    pub position: Position,
}

impl FoundVillage {
    pub fn new(player: Player, position: Position) -> Self {
        Self { player, position }
    }
}

impl Command for FoundVillage {}

pub struct FoundVillageHandler {}

impl Default for FoundVillageHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl FoundVillageHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<FoundVillage> for FoundVillageHandler {
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
mod tests {}
