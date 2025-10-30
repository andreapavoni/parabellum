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
    repository::uow::UnitOfWorkProvider,
};
use commands::*;
use queries::*;

pub struct App {
    config: Arc<Config>,
    uow_provider: Arc<dyn UnitOfWorkProvider>,
}

impl App {
    pub fn new(config: Arc<Config>, uow_provider: Arc<dyn UnitOfWorkProvider>) -> Self {
        Self {
            config,
            uow_provider,
        }
    }

    pub async fn register_player(&self, command: RegisterPlayer) -> Result<Player> {
        let uow = self.uow_provider.begin().await?;
        let handler = RegisterPlayerHandler::new(uow.players());
        match handler.handle(command).await {
            Ok(player) => {
                uow.commit().await?;
                Ok(player)
            }
            Err(e) => {
                uow.rollback().await?;
                Err(e)
            }
        }
    }

    pub async fn found_village(&self, command: FoundVillage) -> Result<Village> {
        let uow = self.uow_provider.begin().await?;
        let handler = FoundVillageHandler::new(uow.villages(), uow.map());
        match handler.handle(command).await {
            Ok(village) => {
                uow.commit().await?;
                Ok(village)
            }
            Err(e) => {
                uow.rollback().await?;
                Err(e)
            }
        }
    }

    pub async fn get_unoccupied_valley(&self, query: GetUnoccupiedValley) -> Result<Valley> {
        let uow = self.uow_provider.begin().await?;
        let handler = GetUnoccupiedValleyHandler::new(uow.map());
        handler.handle(query).await
    }
}
