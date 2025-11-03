use crate::{
    Result,
    cqrs::{Command, CommandHandler},
    db::DbError,
    error::ApplicationError,
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
    ) -> Result<()> {
        let village_id: i32 = command.position.to_id(100) as i32;
        let village_repo: Arc<dyn VillageRepository + '_> = uow.villages();
        let map_repo: Arc<dyn MapRepository + '_> = uow.map();

        let valley = match map_repo.get_field_by_id(village_id).await? {
            Some(map_field) => Valley::try_from(map_field)?,
            None => {
                return Err(ApplicationError::Db(DbError::VillageNotFound(
                    village_id as u32,
                )));
            }
        };

        let village = Village::new("New Village".to_string(), &valley, &command.player, false);

        village_repo.create(&village).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {}
