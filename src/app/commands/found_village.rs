use crate::{
    game::models::{
        map::{Position, Valley},
        village::Village,
        Player,
    },
    repository::{MapRepository, VillageRepository},
};
use anyhow::{anyhow, Result};
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

pub struct FoundVillageHandler<'a> {
    village_repo: Arc<dyn VillageRepository + 'a>,
    map_repo: Arc<dyn MapRepository + 'a>,
}

impl<'a> FoundVillageHandler<'a> {
    pub fn new(
        village_repo: Arc<dyn VillageRepository + 'a>,
        map_repo: Arc<dyn MapRepository + 'a>,
    ) -> Self {
        Self {
            village_repo,
            map_repo,
        }
    }

    pub async fn handle(&self, command: FoundVillage) -> Result<Village> {
        let village_id: i32 = command.position.to_id(100) as i32;

        let valley = match self.map_repo.get_field_by_id(village_id).await? {
            Some(map_field) => Valley::try_from(map_field)?,
            None => return Err(anyhow!("The number of available units is not enough")),
        };

        let village = Village::new("New Village".to_string(), &valley, &command.player, false);

        self.village_repo.create(&village).await?;

        Ok(village)
    }
}
