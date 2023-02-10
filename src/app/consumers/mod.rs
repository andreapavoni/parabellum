mod jobs_consumer;

use anyhow::Result;

use self::jobs_consumer::JobConsumer;
use super::events::GameEvent;

pub trait EventConsumer {
    fn process(event: GameEvent) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct MainConsumer;

impl MainConsumer {
    pub fn process_events(events: Vec<GameEvent>) -> Result<()> {
        for e in events.into_iter() {
            match e {
                GameEvent::VillageFounded(_) => JobConsumer::process(e.clone())?,
                GameEvent::PlayerRegistered(_) => JobConsumer::process(e.clone())?,
                GameEvent::JobEnqueued(_) => JobConsumer::process(e.clone())?,
                GameEvent::ArmyDeployed {
                    army: _,
                    village_id: _,
                } => todo!(),
                GameEvent::TargetAttacked => todo!(),
                GameEvent::TargetRaided => todo!(),
                GameEvent::TargetReinforced => todo!(),
                GameEvent::ArmyReturned => todo!(),
                GameEvent::MerchantArrived => todo!(),
                GameEvent::MerchantReturned => todo!(),
                GameEvent::BarracksUnitTrained => todo!(),
                GameEvent::StableUnitTrained => todo!(),
                GameEvent::WorkshopUnitTrained => todo!(),
                GameEvent::ExpansionUnitTrained => todo!(),
                GameEvent::TrapperUnitTrained => todo!(),
                GameEvent::GreatBarracksUnitTrained => todo!(),
                GameEvent::GreatStableUnitTrained => todo!(),
                GameEvent::GreatWorkshopUnitTrained => todo!(),
                GameEvent::ResearchAcademyCompleted => todo!(),
                GameEvent::ResearchSmithyCompleted => todo!(),
                GameEvent::CelebrationTownHallEnded => todo!(),
                GameEvent::CelebrationBreweryEnded => todo!(),
            };
        }
        Ok(())
    }
}
