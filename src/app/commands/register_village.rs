use crate::{
    app::queries::{GetUnoccupiedValley, GetUnoccupiedValleyHandler},
    game::models::{village::Village, Player},
    repository::{MapRepository, VillageRepository},
};
use anyhow::Result;
use std::sync::Arc;

#[derive(Clone)]
pub struct RegisterVillage {
    pub player: Player,
}

impl RegisterVillage {
    pub fn new(player: Player) -> Self {
        Self { player }
    }
}

pub struct RegisterVillageHandler<'a> {
    village_repo: Arc<dyn VillageRepository + 'a>,
    map_repo: Arc<dyn MapRepository + 'a>,
}

impl<'a> RegisterVillageHandler<'a> {
    pub fn new(
        village_repo: Arc<dyn VillageRepository + 'a>,
        map_repo: Arc<dyn MapRepository + 'a>,
    ) -> Self {
        Self {
            village_repo,
            map_repo,
        }
    }

    pub async fn handle(&self, command: RegisterVillage) -> Result<Village> {
        let query_valley = GetUnoccupiedValley::new(None);
        let handler_vallery = GetUnoccupiedValleyHandler::new(self.map_repo.clone());

        let valley = handler_vallery.handle(query_valley).await?;
        let village = Village::new("New Village".to_string(), &valley, &command.player, true);

        self.village_repo.create(&village).await?;

        Ok(village)
    }
}
