use mini_cqrs_es::EventPayload;
use uuid::Uuid;

use crate::game::models::{village::Village, Player};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    // GameStarted {
    //     aggregate_id: Uuid,
    //     player_1: Player,
    //     player_2: Player,
    //     goal: u32,
    // },
    PlayerRegistered(Player),
    VillageFounded(Village),
    // JobEnqueued(Job),
    // ArmyDeployed { army: Army, village_id: u32 },
    // TargetAttacked,
    // TargetRaided,
    // TargetReinforced,
    // ArmyReturned,
    // MerchantArrived,
    // MerchantReturned,
    // BarracksUnitTrained,
    // StableUnitTrained,
    // WorkshopUnitTrained,
    // ExpansionUnitTrained,
    // TrapperUnitTrained,
    // GreatBarracksUnitTrained,
    // GreatStableUnitTrained,
    // GreatWorkshopUnitTrained,
    // ResearchAcademyCompleted,
    // ResearchSmithyCompleted,
    // CelebrationTownHallEnded,
    // CelebrationBreweryEnded,
    // continue ...
}

wrap_event!(GameEvent);

impl ToString for GameEvent {
    fn to_string(&self) -> String {
        match self {
            // GameEvent::GameStarted { .. } => "GameStarted".to_string(),
            GameEvent::PlayerRegistered(_) => "PlayerRegistered".to_string(),
            GameEvent::VillageFounded(_) => "VillageFounded".to_string(),
        }
    }
}

impl EventPayload for GameEvent {
    fn aggregate_id(&self) -> Uuid {
        match self {
            // GameEvent::GameStarted { aggregate_id, .. } => *aggregate_id,
            GameEvent::PlayerRegistered(player) => *player.aggregate_id,
            GameEvent::VillageFounded(village) => *village.aggregate_id,
        }
    }
}
