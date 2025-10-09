use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::game::{
    battle::CataTargets,
    models::{
        army::{TroopSet, UnitName},
        buildings::BuildingName,
        ResourceGroup,
    },
};

#[derive(Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub task: JobTask,
    pub started_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub done: bool, // ??? if true it means it has been "consumed"
}

impl Job {
    pub fn new(
        player_id: Uuid,
        village_id: u32,
        started_at: DateTime<Utc>,
        ends_at: DateTime<Utc>,
        task: JobTask,
    ) -> Self {
        let id = Uuid::new_v4();

        Self {
            id,
            player_id,
            village_id,
            task,
            started_at,
            ends_at,
            done: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum JobTask {
    Attack {
        units: TroopSet,
        cata_targets: CataTargets,
        village_id: u32,
        player_id: Uuid,
    },
    Raid {
        units: TroopSet,
        village_id: u32,
        player_id: Uuid,
    },
    Reinforcement {
        units: TroopSet,
        village_id: u32,
        player_id: Uuid,
    },
    ArmyReturn {
        units: TroopSet,
        resources: ResourceGroup,
        village_id: u32,
    },

    MerchantGoing {
        resources: ResourceGroup,
        village_id: u32,
        player_id: Uuid,
    },
    MerchantReturn {
        village_id: u32,
    },
    // props: unit_type (Infantry, Cavalry, Siege, Expansion), quantity, building_slot_id, time_for_each_unit? (so it enqueues a new job when 1 unit is finished)
    TrainBarracks {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
    TrainGreatBarracks {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
    TrainStable {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
    TrainGreatStable {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
    TrainWorkshop {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
    TrainGreatWorkshop {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
    TrainExpansion {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },

    BuildingUpgrade {
        slot_id: u8,
        building_name: BuildingName,
    },
    BuildingDowngrade {
        slot_id: u8,
        building_name: BuildingName,
    },

    ResearchAcademy {
        unit: UnitName,
    },
    ResearchSmithy {
        unit: UnitName,
    },

    CelebrationTownHall {
        big: bool,
    },
    CelebrationBrewery,
}
