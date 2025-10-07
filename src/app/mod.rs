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
pub mod job_handlers;
pub mod queries;

use anyhow::Result;
use std::sync::Arc;

use crate::{
    config::Config,
    game::models::{map::Valley, village::Village, Player},
    repository::*,
};
use commands::*;
use queries::*;

pub struct App {
    config: Arc<Config>,
    player_repo: Arc<dyn PlayerRepository>,
    village_repo: Arc<dyn VillageRepository>,
    map_repo: Arc<dyn MapRepository>,
    army_repo: Arc<dyn ArmyRepository>,
    job_repo: Arc<dyn JobRepository>,
}

impl App {
    pub fn new(
        config: Arc<Config>,
        player_repo: Arc<dyn PlayerRepository>,
        village_repo: Arc<dyn VillageRepository>,
        map_repo: Arc<dyn MapRepository>,
        army_repo: Arc<dyn ArmyRepository>,
        job_repo: Arc<dyn JobRepository>,
    ) -> Self {
        Self {
            config,
            army_repo,
            job_repo,
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
