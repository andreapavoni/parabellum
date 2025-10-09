pub mod attack;
pub mod register_player;

use anyhow::Result;

use super::events::GameEvent;
use crate::game::{
    battle::CataTargets,
    models::{army::Army, Tribe},
};

#[async_trait::async_trait]
pub trait Command {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
    async fn run(&self) -> Result<Vec<GameEvent>>;
}

#[derive(Debug, Clone)]
pub enum Cmd {
    RegisterPlayer {
        username: String,
        tribe: Tribe,
    },
    Attack {
        village_id: u32,
        army: Army,
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
