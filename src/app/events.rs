use anyhow::Result;

use super::jobs::Job;
use crate::game::models::army::TroopSet;

pub trait EventStore {
    fn emit(event: GameEvent) -> Result<()>;
    fn load() -> Vec<GameEvent>;
}

#[derive(Debug, Clone)]
pub enum GameEvent {
    JobEnqueued(Job),
    ArmyDeployed { units: TroopSet, village_id: u32 },
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
