use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::game::models::hero::Hero;

use super::{Cost, ResourceGroup, SmithyUpgrades, Tribe};

#[derive(Debug, Clone)]
pub enum UnitRole {
    Infantry,
    Cavalry,
    Scout,
    Ram,
    Cata,
    Chief,
    Settler,
}

#[derive(Debug, Clone)]
pub enum UnitGroup {
    Infantry,
    Cavalry,
    Siege,
    Expansion,
}

#[derive(Debug, Clone)]
pub enum UnitName {
    // --- Romans ---
    Legionnaire,
    Praetorian,
    Imperian,
    EquitesLegati,
    EquitesImperatoris,
    EquitesCaesaris,
    BatteringRam,
    FireCatapult,
    Senator,
    Settler,
    // --- Teutons ---
    Maceman,
    Spearman,
    Axeman,
    Scout,
    Paladin,
    TeutonicKnight,
    Ram,
    Catapult,
    Chief,
    // --- Gauls ---
    Phalanx,
    Swordsman,
    Pathfinder,
    TheutatesThunder,
    Druidrider,
    Haeduan,
    Trebuchet,
    Chieftain,
    // --- Nature ---
    Rat,
    Spider,
    Serpent,
    Bat,
    WildBoar,
    Wolf,
    Bear,
    Crocodile,
    Tiger,
    Elephant,
    // --- Natars ---
    Pikeman,
    ThornedWarrior,
    Guardsman,
    BirdsOfPrey,
    Axerider,
    NatarianKnight,
    Warelephant,
    Ballista,
    NatarianEmperor,
    // // --- Huns ---
    // Mercenary,
    // Bowman,
    // Spotter,
    // SteppeRider,
    // Marksman,
    // Marauder,
    // Logades,
    // // --- Egyptians
    // SlaveMilitia,
    // AshWarden,
    // KhopeshWarrior,
    // SopduExplorer,
    // AnhurGuard,
    // ReshephChariot,
    // StoneCatapult,
    // Nomarch,
    // // --- Spartans ---
    // Hoplite,
    // Sentinel,
    // Shieldsman,
    // TwinsteelTherion,
    // ElpidaRider,
    // CorinthianCrusher,
    // Ephor,
}

type TribeUnits = [Unit; 10];

pub type TroopSet = [u32; 10];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Army {
    pub village_id: u32,
    pub current_map_field_id: Option<u32>, // this army could have been deployed in some other Village or Oasis
    pub player_id: Uuid,
    pub tribe: Tribe,
    pub units: TroopSet,
    pub smithy: SmithyUpgrades,
    /// L'eroe è opzionale, potrebbe non essere presente in tutte le armate.
    pub hero: Option<Hero>,
}

impl Army {
    pub fn new(
        village_id: u32,
        current_map_field_id: Option<u32>,
        player_id: Uuid,
        tribe: Tribe,
        units: TroopSet,
        smithy: SmithyUpgrades,
        hero: Option<Hero>,
    ) -> Self {
        Army {
            village_id,
            player_id,
            tribe,
            units,
            smithy,
            hero,
            current_map_field_id,
        }
    }

    pub fn get_unit(&self, idx: u8) -> Option<Unit> {
        if idx.ge(&0) && idx.lt(&10) {
            Some(get_tribe_units(&self.tribe)[idx as usize].clone())
        } else {
            None
        }
    }

    pub fn unit_amount(&self, idx: u8) -> u32 {
        self.units[idx as usize]
    }

    // Returns the total raw number of troops in the army.
    pub fn immensity(&self) -> u32 {
        self.units.iter().sum()
    }

    pub fn upkeep(&self) -> u32 {
        let units = get_tribe_units(&self.tribe);
        let mut total: u32 = 0;

        for (idx, quantity) in self.units.iter().enumerate() {
            total += units[idx].cost.upkeep * quantity;
        }

        total
    }

    pub fn attack_points(&self) -> (u32, u32) {
        let mut infantry_points: u32 = 0;
        let mut cavalry_points: u32 = 0;

        for (idx, quantity) in self.units.iter().enumerate() {
            let u = self.get_unit(idx as u8).unwrap();

            let smithy_improvement = self.apply_smithy_upgrade(&u, idx, u.attack);

            match u.role {
                UnitRole::Cavalry => cavalry_points += smithy_improvement * quantity,
                _ => infantry_points += smithy_improvement * quantity,
            }
        }
        (infantry_points, cavalry_points)
    }

    pub fn defense_points(&self) -> (u32, u32) {
        let mut infantry_points: u32 = 0;
        let mut cavalry_points: u32 = 0;

        for (idx, quantity) in self.units.into_iter().enumerate() {
            let u = self.get_unit(idx as u8).unwrap();

            let smithy_infantry = self.apply_smithy_upgrade(&u, idx, u.defense_infantry);
            let smithy_cavalry = self.apply_smithy_upgrade(&u, idx, u.defense_cavalry);

            infantry_points += smithy_infantry * quantity;
            cavalry_points += smithy_cavalry * quantity;
        }
        (infantry_points, cavalry_points)
    }

    pub fn scouting_attack_points(&self) -> u32 {
        self.scouting_points(35)
    }

    pub fn scouting_defense_points(&self) -> u32 {
        self.scouting_points(20)
    }

    pub fn apply_losses(&mut self, percent: f64) {
        for (idx, quantity) in self.units.into_iter().enumerate() {
            self.units[idx] = quantity - ((quantity as f64) * percent / 100.0).floor() as u32;
        }
    }

    pub fn calculate_losses(&self, percent: f64) -> (TroopSet, TroopSet) {
        let mut survivors: TroopSet = [0; 10];
        let mut losses: TroopSet = [0; 10];

        for (idx, quantity) in self.units.into_iter().enumerate() {
            let lost = ((quantity as f64) * percent / 100.0).floor() as u32;
            survivors[idx] = quantity - lost;
            losses[idx] = lost;
        }

        (survivors, losses)
    }

    // Returns a new Army which has been extracted from the current one.
    pub fn deploy(&mut self, set: TroopSet) -> Result<TroopSet> {
        for (idx, quantity) in set.into_iter().enumerate() {
            if self.units[idx] > quantity {
                self.units[idx] -= quantity;
            } else {
                return Err(anyhow!("The number of available units is not enough"));
            }
        }
        Ok(set.clone())
    }

    // Returns the actual speed of the Army by taking the speed of slowest unit.
    pub fn speed(&self) -> u8 {
        let mut speed: u8 = 0;
        for (idx, quantity) in self.units.into_iter().enumerate() {
            if quantity > 0 {
                let u = self.get_unit(idx as u8).unwrap();
                if u.speed < speed {
                    speed = u.speed;
                }
            }
        }
        speed
    }

    pub fn get_troop_count_by_role(&self, role: UnitRole) -> u32 {
        self.units
            .iter()
            .enumerate()
            .filter(|(idx, &quantity)| {
                if quantity > 0 {
                    let unit = self.get_unit(*idx as u8).unwrap();
                    return std::mem::discriminant(&unit.role) == std::mem::discriminant(&role);
                }
                false
            })
            .map(|(_, &q)| q)
            .sum()
    }

    fn scouting_points(&self, base_points: u8) -> u32 {
        let idx: usize = 3;
        let quantity = self.units[idx];
        let unit = self.get_unit(idx as u8).unwrap();
        let smithy_improvement = self.apply_smithy_upgrade(&unit, idx, base_points as u32);
        smithy_improvement * quantity
    }

    fn apply_smithy_upgrade(&self, unit: &Unit, idx: usize, combat_value: u32) -> u32 {
        let level: i32 = self.smithy[idx as usize].into();
        ((combat_value as f64)
            + ((combat_value + 300 * unit.cost.upkeep) as f64 / 7.0)
                * ((1.007f64).powi(level.try_into().unwrap()) - 1.0).floor()) as u32
    }
}

#[derive(Debug, Clone)]
pub struct Unit {
    pub name: UnitName,
    pub role: UnitRole,
    pub group: UnitGroup,
    pub attack: u32,
    pub defense_infantry: u32,
    pub defense_cavalry: u32,
    pub speed: u8,
    pub capacity: u32,
    pub cost: Cost,
}

impl Unit {
    pub fn apply_smithy_upgrade(&self, smithy_level: i32, upkeep: u32, combat_value: u32) -> u32 {
        ((combat_value as f64)
            + ((combat_value + 300 * upkeep) as f64 / 7.0)
                * ((1.007f64).powi(smithy_level.try_into().unwrap()) - 1.0).floor()) as u32
    }
}

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
            build_time: 533,
        },
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
            build_time: 597,
        },
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
            build_time: 640,
        },
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
            build_time: 453,
        },
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
            build_time: 880,
        },
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
            build_time: 1173,
        },
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
            build_time: 1533,
        },
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
            build_time: 3000,
        },
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
            build_time: 30233,
        },
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
            build_time: 8967,
        },
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
            build_time: 240,
        },
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
            build_time: 73,
        },
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
            build_time: 76,
        },
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
            build_time: 73,
        },
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
            build_time: 800,
        },
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
            build_time: 987,
        },
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
            build_time: 1400,
        },
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
            build_time: 3000,
        },
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
            build_time: 23500,
        },
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
            build_time: 10333,
        },
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
            build_time: 347,
        },
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
            build_time: 480,
        },
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
            build_time: 75,
        },
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
            build_time: 827,
        },
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
            build_time: 853,
        },
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
            build_time: 1040,
        },
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
            build_time: 1667,
        },
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
            build_time: 3000,
        },
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
            resources: ResourceGroup::new(30750, 45400, 31000, 37500),
            upkeep: 4,
            build_time: 30233,
        },
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
            build_time: 7567,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
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
            build_time: 0,
        },
    },
];

fn get_tribe_units(tribe: &Tribe) -> &TribeUnits {
    match tribe {
        Tribe::Roman => &ROMAN_UNITS,
        Tribe::Gaul => &GAUL_UNITS,
        Tribe::Teuton => &TEUTON_UNITS,
        Tribe::Nature => &NATURE_UNITS,
        Tribe::Natar => &NATAR_UNITS,
    }
}
