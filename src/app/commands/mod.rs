mod attack;

use anyhow::Result;

use self::attack::AttackCommand;
use super::{consumers::MainConsumer, events::GameEvent};
use crate::game::{battle::CataTargets, models::army::TroopSet};

pub trait Command {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
    fn run(&self) -> Result<Vec<GameEvent>>;
}

#[derive(Debug, Clone)]
pub enum Cmd {
    Attack {
        village_id: u32,
        units: TroopSet,
        cata_targets: CataTargets,
        defender_map_id: u32,
    },
    Raid,
    Reinforce,
    ReturnArmy,
    SendMerchant,
    ReturnMerchant,
    TrainBarracksUnit,
    TrainStableUnit,
    TrainWorkshopUnit,
    TrainExpansionUnit,
    TrainTrapperUnit,
    TrainGreatBarracksUnit,
    TrainGreatStableUnit,
    TrainGreatWorkshopUnit,
    ResearchAcademy,
    ResearchSmithy,
    StartTownHallCelebration,
    StartBreweryCelebration,
}

#[derive(Debug, Clone)]
pub struct Commander;

impl Commander {
    pub fn run(cmd: Cmd) -> Result<()> {
        // run commands, get events
        let command: Box<dyn Command> = match cmd {
            Cmd::Attack {
                village_id,
                units,
                cata_targets,
                defender_map_id: defender_village_id,
            } => Box::new(AttackCommand::new(
                village_id,
                units.clone(),
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

        command.validate()?;
        let events = command.run()?;

        MainConsumer::process_events(events)
    }
}
