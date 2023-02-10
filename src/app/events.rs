use anyhow::Result;

use super::jobs::Job;
use crate::game::models::{army::Army, village::Village, Player};

pub trait EventStore {
    fn emit(event: GameEvent) -> Result<()>;
    fn load() -> Vec<GameEvent>;
}

#[derive(Debug, Clone)]
pub enum GameEvent {
    PlayerRegistered(Player),
    VillageFounded(Village),
    JobEnqueued(Job),
    ArmyDeployed { army: Army, village_id: u32 },
    TargetAttacked,
    TargetRaided,
    TargetReinforced,
    ArmyReturned,
    MerchantArrived,
    MerchantReturned,
    BarracksUnitTrained,
    StableUnitTrained,
    WorkshopUnitTrained,
    ExpansionUnitTrained,
    TrapperUnitTrained,
    GreatBarracksUnitTrained,
    GreatStableUnitTrained,
    GreatWorkshopUnitTrained,
    ResearchAcademyCompleted,
    ResearchSmithyCompleted,
    CelebrationTownHallEnded,
    CelebrationBreweryEnded,
}
