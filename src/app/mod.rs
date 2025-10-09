// RegisterPlayer
// Attack
// Raid
// Reinforce
// ReturnArmy
// SendMerchant
// ReturnMerchant
// TrainBarracksUnit
// TrainStableUnit
// TrainWorkshopUnit
// TrainExpansionUnit
// TrainTrapperUnit
// TrainGreatBarracksUnit
// TrainGreatStableUnit
// TrainGreatWorkshopUnit
// ResearchAcademy
// ResearchSmithy
// StartTownHallCelebration
// StartBreweryCelebration

pub mod commands;
pub mod jobs;
pub mod queries;

use anyhow::Result;
use std::sync::Arc;

use crate::{
    game::models::{map::Valley, village::Village, Player},
    repository::*,
};
use commands::*;
use queries::*;

pub struct App {
    player_repo: Arc<dyn PlayerRepository>,
    village_repo: Arc<dyn VillageRepository>,
    map_repo: Arc<dyn MapRepository>,
}

impl App {
    pub fn new(
        player_repo: Arc<dyn PlayerRepository>,
        village_repo: Arc<dyn VillageRepository>,
        map_repo: Arc<dyn MapRepository>,
    ) -> Self {
        Self {
            player_repo,
            village_repo,
            map_repo,
        }
    }

    pub async fn register_player(&self, command: RegisterPlayer) -> Result<Player> {
        let handler = RegisterPlayerHandler::new(self.player_repo.clone());
        handler.handle(command).await
    }

    pub async fn found_village(&self, command: FoundVillage) -> Result<Village> {
        let handler = FoundVillageHandler::new(self.village_repo.clone(), self.map_repo.clone());
        handler.handle(command).await
    }

    pub async fn get_unoccupied_valley(&self, query: GetUnoccupiedValley) -> Result<Valley> {
        let handler = GetUnoccupiedValleyHandler::new(self.map_repo.clone());
        handler.handle(query).await
    }
}
