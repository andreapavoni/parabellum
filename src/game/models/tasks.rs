use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::game::battle::CataTargets;

use super::{
    army::{TroopSet, UnitName},
    buildings::BuildingName,
    common::ResourceGroup,
};

#[derive(Debug, Clone)]
pub enum TaskJob {
    ArmyMovement(ArmyMovementTask),
    MerchantMovement(MerchantMovementTask),
    TrainArmyUnits(TrainArmyUnitsTask),
    UpdateBuilding(UpdateBuildingTask),
    Research(ResearchTask),
    Celebration(CelebrationTask),
}

#[derive(Debug, Clone)]
pub enum ArmyMovementTask {
    Attack {
        units: TroopSet,
        cata_targets: CataTargets,
        village_id: u64,
        player_id: String,
    },
    Raid {
        units: TroopSet,
        village_id: u64,
        player_id: String,
    },
    Reinforcement {
        units: TroopSet,
        village_id: u64,
        player_id: String,
    },
    Return {
        units: TroopSet,
        resources: ResourceGroup,
        village_id: u64,
    },
}

#[derive(Debug, Clone)]
pub enum MerchantMovementTask {
    Going {
        resources: ResourceGroup,
        village_id: u64,
        player_id: String,
    },
    Return {
        village_id: u64,
    },
}

#[derive(Debug, Clone)]
pub enum TrainArmyUnitsTask {
    // props: unit_type (Infantry, Cavalry, Siege, Expansion), quantity, building_slot_id, time_for_each_unit? (so it enqueues a new job when 1 unit is finished)
    Barracks {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
    GreatBarracks {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
    Stable {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
    GreatStable {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
    Workshop {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
    GreatWorkshop {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
    ResidenceOrPalace {
        slot_id: u8,
        unit: UnitName,
        quantity: u32,
        time_per_unit_secs: u32,
    },
}

#[derive(Debug, Clone)]
pub enum UpdateBuildingTask {
    Upgrade {
        slot_id: u8,
        building_name: BuildingName,
    },
    Downgrade {
        slot_id: u8,
        building_name: BuildingName,
    },
}

#[derive(Debug, Clone)]
pub enum ResearchTask {
    Academy { unit: UnitName },
    Smithy { unit: UnitName },
}

#[derive(Debug, Clone)]
pub enum CelebrationTask {
    TownHall { big: bool },
    Brewery,
}

#[derive(Debug, Clone)]
pub struct TaskItem {
    pub id: Uuid,
    pub player_id: String,
    pub village_id: u64,
    pub job: TaskJob,
    pub started_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub done: bool, // ??? if true it measn it has been "consumed"
}

impl TaskItem {
    pub fn new(
        player_id: String,
        village_id: u64,
        started_at: DateTime<Utc>,
        ends_at: DateTime<Utc>,
        job: TaskJob,
    ) -> Self {
        let id = Uuid::new_v4();

        Self {
            id,
            player_id,
            village_id,
            job,
            started_at,
            ends_at,
            done: false,
        }
    }
}
