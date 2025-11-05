use serde::{Deserialize, Serialize};

use super::{
    army::{Unit, UnitGroup, UnitName, UnitRequirement, UnitRole},
    buildings::BuildingName,
    common::{Cost, ResearchCost, ResourceGroup},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub enum Tribe {
    Roman,
    Gaul,
    Teuton,
    Natar,
    Nature,
}

impl Tribe {
    pub fn get_units(&self) -> &TribeUnits {
        match self {
            Tribe::Roman => &ROMAN_UNITS,
            Tribe::Gaul => &GAUL_UNITS,
            Tribe::Teuton => &TEUTON_UNITS,
            Tribe::Nature => &NATURE_UNITS,
            Tribe::Natar => &NATAR_UNITS,
        }
    }

    pub fn get_unit_idx_by_name(&self, unit_name: &UnitName) -> Option<usize> {
        self.get_units()
            .iter()
            .position(|unit| unit.name == *unit_name)
    }
}

pub type TribeUnits = [Unit; 10];

static ROMAN_UNITS: TribeUnits = [
    Unit {
        name: UnitName::Legionnaire,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 40,
        defense_infantry: 35,
        defense_cavalry: 50,
        speed: 12,
        capacity: 50,
        cost: Cost {
            resources: ResourceGroup::new(120, 100, 150, 30),
            upkeep: 1,
            time: 533,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[UnitRequirement {
            building: BuildingName::Barracks,
            level: 1,
        }],
    },
    Unit {
        name: UnitName::Praetorian,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 30,
        defense_infantry: 65,
        defense_cavalry: 35,
        speed: 10,
        capacity: 20,
        cost: Cost {
            resources: ResourceGroup::new(100, 130, 160, 70),
            upkeep: 1,
            time: 597,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(700, 620, 1480, 580),
            time: 8400,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 1,
            },
            UnitRequirement {
                building: BuildingName::Smithy,
                level: 1,
            },
        ],
    },
    Unit {
        name: UnitName::Imperian,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 70,
        defense_infantry: 40,
        defense_cavalry: 25,
        speed: 14,
        capacity: 50,
        cost: Cost {
            resources: ResourceGroup::new(150, 160, 210, 80),
            upkeep: 1,
            time: 640,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(1000, 740, 1880, 640),
            time: 9000,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 5,
            },
            UnitRequirement {
                building: BuildingName::Smithy,
                level: 1,
            },
        ],
    },
    Unit {
        name: UnitName::EquitesLegati,
        role: UnitRole::Scout,
        group: UnitGroup::Cavalry,
        attack: 0,
        defense_infantry: 20,
        defense_cavalry: 10,
        speed: 32,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(140, 160, 20, 40),
            upkeep: 2,
            time: 453,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(940, 740, 360, 400),
            time: 6900,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 5,
            },
            UnitRequirement {
                building: BuildingName::Stable,
                level: 1,
            },
        ],
    },
    Unit {
        name: UnitName::EquitesImperatoris,
        role: UnitRole::Cavalry,
        group: UnitGroup::Cavalry,
        attack: 120,
        defense_infantry: 65,
        defense_cavalry: 50,
        speed: 28,
        capacity: 100,
        cost: Cost {
            resources: ResourceGroup::new(550, 440, 320, 100),
            upkeep: 3,
            time: 880,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(3400, 1860, 2760, 760),
            time: 11700,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 5,
            },
            UnitRequirement {
                building: BuildingName::Stable,
                level: 5,
            },
        ],
    },
    Unit {
        name: UnitName::EquitesCaesaris,
        role: UnitRole::Cavalry,
        group: UnitGroup::Cavalry,
        attack: 180,
        defense_infantry: 80,
        defense_cavalry: 105,
        speed: 20,
        capacity: 70,
        cost: Cost {
            resources: ResourceGroup::new(550, 640, 800, 180),
            upkeep: 4,
            time: 1173,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(3400, 2660, 6600, 1240),
            time: 15000,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 5,
            },
            UnitRequirement {
                building: BuildingName::Stable,
                level: 5,
            },
        ],
    },
    Unit {
        name: UnitName::BatteringRam,
        role: UnitRole::Ram,
        group: UnitGroup::Siege,
        attack: 60,
        defense_infantry: 30,
        defense_cavalry: 75,
        speed: 8,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(900, 360, 500, 70),
            upkeep: 3,
            time: 1533,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(5500, 1540, 4200, 580),
            time: 15600,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 10,
            },
            UnitRequirement {
                building: BuildingName::Workshop,
                level: 1,
            },
        ],
    },
    Unit {
        name: UnitName::FireCatapult,
        role: UnitRole::Cata,
        group: UnitGroup::Siege,
        attack: 75,
        defense_infantry: 60,
        defense_cavalry: 10,
        speed: 6,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(950, 1350, 600, 90),
            upkeep: 6,
            time: 3000,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(5800, 5500, 5000, 700),
            time: 28800,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 15,
            },
            UnitRequirement {
                building: BuildingName::Workshop,
                level: 10,
            },
        ],
    },
    Unit {
        name: UnitName::Senator,
        role: UnitRole::Chief,
        group: UnitGroup::Expansion,
        attack: 50,
        defense_infantry: 40,
        defense_cavalry: 30,
        speed: 8,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(30750, 27200, 45000, 37500),
            upkeep: 5,
            time: 30233,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(15880, 13800, 36400, 22660),
            time: 24475,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 20,
            },
            UnitRequirement {
                building: BuildingName::RallyPoint,
                level: 10,
            },
        ],
    },
    Unit {
        name: UnitName::Settler,
        role: UnitRole::Settler,
        group: UnitGroup::Expansion,
        attack: 0,
        defense_infantry: 80,
        defense_cavalry: 80,
        speed: 10,
        capacity: 3000,
        cost: Cost {
            resources: ResourceGroup::new(4600, 4200, 5800, 4400),
            upkeep: 1,
            time: 8967,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
];

static TEUTON_UNITS: TribeUnits = [
    Unit {
        name: UnitName::Maceman,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 40,
        defense_infantry: 20,
        defense_cavalry: 5,
        speed: 14,
        capacity: 60,
        cost: Cost {
            resources: ResourceGroup::new(95, 75, 40, 40),
            upkeep: 1,
            time: 240,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[UnitRequirement {
            building: BuildingName::Barracks,
            level: 1,
        }],
    },
    Unit {
        name: UnitName::Spearman,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 10,
        defense_infantry: 35,
        defense_cavalry: 60,
        speed: 14,
        capacity: 40,
        cost: Cost {
            resources: ResourceGroup::new(145, 70, 85, 40),
            upkeep: 1,
            time: 73,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(970, 380, 880, 400),
            time: 5760,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 1,
            },
            UnitRequirement {
                building: BuildingName::Barracks,
                level: 3,
            },
        ],
    },
    Unit {
        name: UnitName::Axeman,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 60,
        defense_infantry: 30,
        defense_cavalry: 30,
        speed: 12,
        capacity: 50,
        cost: Cost {
            resources: ResourceGroup::new(130, 120, 170, 70),
            upkeep: 1,
            time: 76,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(880, 580, 1560, 580),
            time: 6300,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 3,
            },
            UnitRequirement {
                building: BuildingName::Smithy,
                level: 1,
            },
        ],
    },
    Unit {
        name: UnitName::Scout,
        role: UnitRole::Scout,
        group: UnitGroup::Infantry,
        attack: 0,
        defense_infantry: 10,
        defense_cavalry: 5,
        speed: 18,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(160, 100, 50, 50),
            upkeep: 1,
            time: 73,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(1060, 500, 600, 460),
            time: 6000,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 1,
            },
            UnitRequirement {
                building: BuildingName::MainBuilding,
                level: 5,
            },
        ],
    },
    Unit {
        name: UnitName::Paladin,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 55,
        defense_infantry: 100,
        defense_cavalry: 40,
        speed: 20,
        capacity: 110,
        cost: Cost {
            resources: ResourceGroup::new(370, 270, 290, 75),
            upkeep: 2,
            time: 800,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(2320, 1180, 2520, 610),
            time: 10800,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 5,
            },
            UnitRequirement {
                building: BuildingName::Stable,
                level: 3,
            },
        ],
    },
    Unit {
        name: UnitName::TeutonicKnight,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 150,
        defense_infantry: 50,
        defense_cavalry: 75,
        speed: 18,
        capacity: 80,
        cost: Cost {
            resources: ResourceGroup::new(450, 515, 480, 80),
            upkeep: 3,
            time: 987,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(2800, 2160, 4040, 640),
            time: 13500,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 15,
            },
            UnitRequirement {
                building: BuildingName::Stable,
                level: 10,
            },
        ],
    },
    Unit {
        name: UnitName::Ram,
        role: UnitRole::Ram,
        group: UnitGroup::Siege,
        attack: 65,
        defense_infantry: 30,
        defense_cavalry: 80,
        speed: 8,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(1000, 300, 350, 70),
            upkeep: 3,
            time: 1400,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(6100, 1300, 3000, 580),
            time: 14400,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 10,
            },
            UnitRequirement {
                building: BuildingName::Workshop,
                level: 1,
            },
        ],
    },
    Unit {
        name: UnitName::Catapult,
        role: UnitRole::Cata,
        group: UnitGroup::Siege,
        attack: 50,
        defense_infantry: 60,
        defense_cavalry: 10,
        speed: 6,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(900, 1200, 600, 60),
            upkeep: 6,
            time: 3000,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(5500, 4900, 5000, 520),
            time: 28800,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 15,
            },
            UnitRequirement {
                building: BuildingName::Workshop,
                level: 10,
            },
        ],
    },
    Unit {
        name: UnitName::Chief,
        role: UnitRole::Chief,
        group: UnitGroup::Expansion,
        attack: 40,
        defense_infantry: 60,
        defense_cavalry: 40,
        speed: 8,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(35500, 26600, 25000, 27200),
            upkeep: 4,
            time: 23500,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(18250, 13500, 20400, 16480),
            time: 19425,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 20,
            },
            UnitRequirement {
                building: BuildingName::RallyPoint,
                level: 5,
            },
        ],
    },
    Unit {
        name: UnitName::Settler,
        role: UnitRole::Settler,
        group: UnitGroup::Expansion,
        attack: 10,
        defense_infantry: 80,
        defense_cavalry: 80,
        speed: 10,
        capacity: 3000,
        cost: Cost {
            resources: ResourceGroup::new(5800, 4400, 4600, 5200),
            upkeep: 1,
            time: 10333,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
];

static GAUL_UNITS: TribeUnits = [
    Unit {
        name: UnitName::Phalanx,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 15,
        defense_infantry: 40,
        defense_cavalry: 50,
        speed: 14,
        capacity: 35,
        cost: Cost {
            resources: ResourceGroup::new(100, 130, 55, 30),
            upkeep: 1,
            time: 347,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[UnitRequirement {
            building: BuildingName::Barracks,
            level: 1,
        }],
    },
    Unit {
        name: UnitName::Swordsman,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 65,
        defense_infantry: 35,
        defense_cavalry: 20,
        speed: 12,
        capacity: 45,
        cost: Cost {
            resources: ResourceGroup::new(140, 150, 185, 60),
            upkeep: 1,
            time: 480,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(940, 700, 1689, 520),
            time: 7200,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 3,
            },
            UnitRequirement {
                building: BuildingName::Smithy,
                level: 1,
            },
        ],
    },
    Unit {
        name: UnitName::Pathfinder,
        role: UnitRole::Scout,
        group: UnitGroup::Cavalry,
        attack: 0,
        defense_infantry: 20,
        defense_cavalry: 10,
        speed: 34,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(170, 150, 20, 40),
            upkeep: 2,
            time: 75,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(1120, 700, 360, 400),
            time: 3501,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 5,
            },
            UnitRequirement {
                building: BuildingName::Stable,
                level: 1,
            },
        ],
    },
    Unit {
        name: UnitName::TheutatesThunder,
        role: UnitRole::Cavalry,
        group: UnitGroup::Cavalry,
        attack: 100,
        defense_infantry: 25,
        defense_cavalry: 40,
        speed: 38,
        capacity: 75,
        cost: Cost {
            resources: ResourceGroup::new(350, 450, 230, 60),
            upkeep: 2,
            time: 827,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(2200, 1900, 2040, 520),
            time: 11100,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 5,
            },
            UnitRequirement {
                building: BuildingName::Stable,
                level: 3,
            },
        ],
    },
    Unit {
        name: UnitName::Druidrider,
        role: UnitRole::Cavalry,
        group: UnitGroup::Cavalry,
        attack: 45,
        defense_infantry: 115,
        defense_cavalry: 55,
        speed: 32,
        capacity: 35,
        cost: Cost {
            resources: ResourceGroup::new(360, 330, 280, 120),
            upkeep: 2,
            time: 853,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(2260, 1420, 2440, 880),
            time: 11400,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 5,
            },
            UnitRequirement {
                building: BuildingName::Stable,
                level: 5,
            },
        ],
    },
    Unit {
        name: UnitName::Haeduan,
        role: UnitRole::Cavalry,
        group: UnitGroup::Cavalry,
        attack: 140,
        defense_infantry: 60,
        defense_cavalry: 165,
        speed: 26,
        capacity: 65,
        cost: Cost {
            resources: ResourceGroup::new(500, 620, 675, 170),
            upkeep: 3,
            time: 1040,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(3100, 2580, 5600, 1180),
            time: 13500,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 15,
            },
            UnitRequirement {
                building: BuildingName::Stable,
                level: 10,
            },
        ],
    },
    Unit {
        name: UnitName::Ram,
        role: UnitRole::Ram,
        group: UnitGroup::Siege,
        attack: 50,
        defense_infantry: 30,
        defense_cavalry: 105,
        speed: 8,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(950, 555, 330, 75),
            upkeep: 3,
            time: 1667,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(5800, 2320, 2840, 610),
            time: 16800,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 10,
            },
            UnitRequirement {
                building: BuildingName::Workshop,
                level: 1,
            },
        ],
    },
    Unit {
        name: UnitName::Trebuchet,
        role: UnitRole::Cata,
        group: UnitGroup::Siege,
        attack: 70,
        defense_infantry: 45,
        defense_cavalry: 10,
        speed: 6,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(960, 1450, 630, 90),
            upkeep: 6,
            time: 3000,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(5860, 5900, 5240, 700),
            time: 28800,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 15,
            },
            UnitRequirement {
                building: BuildingName::Workshop,
                level: 10,
            },
        ],
    },
    Unit {
        name: UnitName::Chieftain,
        role: UnitRole::Chief,
        group: UnitGroup::Expansion,
        attack: 40,
        defense_infantry: 50,
        defense_cavalry: 50,
        speed: 10,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(15880, 22900, 25200, 22660),
            upkeep: 4,
            time: 30233,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(1060, 500, 600, 460),
            time: 24475,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 20,
            },
            UnitRequirement {
                building: BuildingName::RallyPoint,
                level: 10,
            },
        ],
    },
    Unit {
        name: UnitName::Settler,
        role: UnitRole::Settler,
        group: UnitGroup::Expansion,
        attack: 0,
        defense_infantry: 80,
        defense_cavalry: 80,
        speed: 10,
        capacity: 3000,
        cost: Cost {
            resources: ResourceGroup::new(4400, 5600, 4200, 3900),
            upkeep: 1,
            time: 7567,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(1060, 500, 600, 460),
            time: 6000,
        },
        requirements: &[
            UnitRequirement {
                building: BuildingName::Academy,
                level: 1,
            },
            UnitRequirement {
                building: BuildingName::MainBuilding,
                level: 5,
            },
        ],
    },
];

static NATURE_UNITS: TribeUnits = [
    Unit {
        name: UnitName::Rat,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 10,
        defense_infantry: 25,
        defense_cavalry: 20,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 100, 100),
            upkeep: 1,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Spider,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 20,
        defense_infantry: 35,
        defense_cavalry: 40,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 1,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Serpent,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 60,
        defense_infantry: 40,
        defense_cavalry: 60,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 1,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Bat,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 80,
        defense_infantry: 66,
        defense_cavalry: 50,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 1,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::WildBoar,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 50,
        defense_infantry: 70,
        defense_cavalry: 33,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 2,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Wolf,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 100,
        defense_infantry: 80,
        defense_cavalry: 70,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 2,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Bear,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 250,
        defense_infantry: 140,
        defense_cavalry: 200,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 3,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Crocodile,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 450,
        defense_infantry: 380,
        defense_cavalry: 240,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 3,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Tiger,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 200,
        defense_infantry: 170,
        defense_cavalry: 250,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 3,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Elephant,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 600,
        defense_infantry: 440,
        defense_cavalry: 520,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 5,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
];

static NATAR_UNITS: TribeUnits = [
    Unit {
        name: UnitName::Pikeman,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 20,
        defense_infantry: 35,
        defense_cavalry: 50,
        speed: 12,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 1,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::ThornedWarrior,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 65,
        defense_infantry: 30,
        defense_cavalry: 10,
        speed: 14,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 1,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Guardsman,
        role: UnitRole::Infantry,
        group: UnitGroup::Infantry,
        attack: 100,
        defense_infantry: 90,
        defense_cavalry: 75,
        speed: 12,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 1,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::BirdsOfPrey,
        role: UnitRole::Scout,
        group: UnitGroup::Infantry,
        attack: 0,
        defense_infantry: 10,
        defense_cavalry: 0,
        speed: 50,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 1,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Axerider,
        role: UnitRole::Cavalry,
        group: UnitGroup::Cavalry,
        attack: 155,
        defense_infantry: 80,
        defense_cavalry: 50,
        speed: 28,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 2,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::NatarianKnight,
        role: UnitRole::Cavalry,
        group: UnitGroup::Cavalry,
        attack: 170,
        defense_infantry: 140,
        defense_cavalry: 80,
        speed: 24,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 3,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Warelephant,
        role: UnitRole::Ram,
        group: UnitGroup::Siege,
        attack: 250,
        defense_infantry: 120,
        defense_cavalry: 150,
        speed: 10,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 4,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Ballista,
        role: UnitRole::Cata,
        group: UnitGroup::Siege,
        attack: 60,
        defense_infantry: 45,
        defense_cavalry: 10,
        speed: 6,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 5,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::NatarianEmperor,
        role: UnitRole::Chief,
        group: UnitGroup::Expansion,
        attack: 80,
        defense_infantry: 50,
        defense_cavalry: 50,
        speed: 10,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 1,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
    Unit {
        name: UnitName::Settler,
        role: UnitRole::Settler,
        group: UnitGroup::Expansion,
        attack: 30,
        defense_infantry: 40,
        defense_cavalry: 40,
        speed: 10,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            upkeep: 1,
            time: 0,
        },
        research_cost: ResearchCost {
            resources: ResourceGroup::new(0, 0, 0, 0),
            time: 0,
        },
        requirements: &[],
    },
];
