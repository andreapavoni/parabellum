use std::sync::Arc;

use anyhow::Result;

use crate::repository::Repository;

use self::{
    commands::{attack::AttackCommand, register_player::RegisterPlayerCommand, Cmd, Command},
    consumers::MainConsumer,
};

pub mod aggregates;
pub mod commands;
pub mod consumers;
pub mod events;
pub mod jobs;

pub struct App {
    repo: Arc<dyn Repository>,
}

impl App {
    pub fn new(repo: Arc<dyn Repository>) -> Self {
        Self { repo }
    }

    pub async fn command(&self, cmd: Cmd) -> Result<()> {
        let command: Box<dyn Command> = match cmd {
            Cmd::RegisterPlayer { username, tribe } => Box::new(RegisterPlayerCommand::new(
                self.repo.clone(),
                username,
                tribe,
            )),
            Cmd::Attack {
                village_id,
                army,
                cata_targets,
                defender_map_id: defender_village_id,
            } => Box::new(AttackCommand::new(
                self.repo.clone(),
                village_id,
                army.clone(),
                cata_targets.clone(),
                defender_village_id,
            )),
            Cmd::Raid => todo!(),
            Cmd::Reinforce => todo!(),
            Cmd::ReturnArmy => todo!(),
            Cmd::SendMerchant => todo!(),
            Cmd::ReturnMerchant => todo!(),
            Cmd::TrainBarracksUnit => todo!(),
            Cmd::TrainStableUnit => todo!(),
            Cmd::TrainWorkshopUnit => todo!(),
            Cmd::TrainExpansionUnit => todo!(),
            Cmd::TrainTrapperUnit => todo!(),
            Cmd::TrainGreatBarracksUnit => todo!(),
            Cmd::TrainGreatStableUnit => todo!(),
            Cmd::TrainGreatWorkshopUnit => todo!(),
            Cmd::ResearchAcademy => todo!(),
            Cmd::ResearchSmithy => todo!(),
            Cmd::StartTownHallCelebration => todo!(),
            Cmd::StartBreweryCelebration => todo!(),
        };

        // command.validate()?;
        let events = command.run().await?;

        println!("Produced events -> {:?}", events.clone());

        MainConsumer::process_events(events)?;

        Ok(())
    }
}
