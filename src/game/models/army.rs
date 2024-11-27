use anyhow::Result;

use super::common::{Cost, ResourceGroup, SmithyUpgrades, Tribe};

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
pub enum UnitName {
    // Romans
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
    // Teutons
    Maceman,
    Spearman,
    Axeman,
    Scout,
    Paladin,
    TeutonicKnight,
    Ram,
    Catapult,
    Chief,
    // Gauls
    Phalanx,
    Swordsman,
    Pathfinder,
    TheutatesThunder,
    Druidrider,
    Haeduan,
    Trebuchet,
    Chieftain,
    // Nature
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
    // Natars
    Pikeman,
    ThornedWarrior,
    Guardsman,
    BirdsOfPrey,
    Axerider,
    NatarianKnight,
    Warelephant,
    Ballista,
    NatarianEmperor,
    // Huns
    Mercenary,
    Bowman,
    Spotter,
    SteppeRider,
    Marksman,
    Marauder,
    Logades,
    // Egyptians
    SlaveMilitia,
    AshWarden,
    KhopeshWarrior,
    SopduExplorer,
    AnhurGuard,
    ReshephChariot,
    StoneCatapult,
    Nomarch,
    // Spartans
    Hoplite,
    Sentinel,
    Shieldsman,
    TwinsteelTherion,
    ElpidaRider,
    CorinthianCrusher,
    Ephor,
}

type TribeUnits = [Unit; 10];
pub type TroopSet = [u64; 10];

#[derive(Debug, Clone)]
pub struct Army {
    pub village_id: u64,
    pub player_id: String,
    pub tribe: Tribe,
    pub units: TroopSet,
    pub smithy: SmithyUpgrades,
}

impl Army {
    pub fn new(
        village_id: u64,
        player_id: String,
        tribe: Tribe,
        units: TroopSet,
        smithy: SmithyUpgrades,
    ) -> Self {
        Army {
            village_id,
            player_id,
            tribe,
            units,
            smithy,
        }
    }

    pub fn get_unit(&self, idx: u8) -> Result<Unit> {
        Ok(get_tribe_units(self.tribe.clone())[idx as usize].clone())
    }

    pub fn unit_amount(&self, idx: u8) -> u64 {
        self.units[idx as usize]
    }

    // Returns the total raw number of troops in the army.
    pub fn immensity(&self) -> u64 {
        self.units.into_iter().sum()
    }

    pub fn upkeep(&self) -> u64 {
        let units = get_tribe_units(self.tribe.clone());
        let mut total: u64 = 0;

        units.into_iter().for_each(|u| {
            total += u.cost.upkeep;
        });

        total
    }

    pub fn attack_points(&self) -> (u64, u64) {
        let mut infantry_points: u64 = 0;
        let mut cavalry_points: u64 = 0;

        for (idx, quantity) in self.units.into_iter().enumerate() {
            let u = self.get_unit(idx.try_into().unwrap()).unwrap();

            let smithy_improvement = self.apply_smithy_upgrade(u.clone(), idx, u.attack);

            match u.role {
                UnitRole::Cavalry => cavalry_points += smithy_improvement * quantity,
                _ => infantry_points += smithy_improvement * quantity,
            }
        }
        (infantry_points, cavalry_points)
    }

    pub fn defense_points(&self) -> (u64, u64) {
        let mut infantry_points: u64 = 0;
        let mut cavalry_points: u64 = 0;

        for (idx, quantity) in self.units.into_iter().enumerate() {
            let u = self.get_unit(idx.try_into().unwrap()).unwrap();

            let smithy_infantry = self.apply_smithy_upgrade(u.clone(), idx, u.defense_infantry);
            let smithy_cavalry = self.apply_smithy_upgrade(u.clone(), idx, u.defense_cavalry);

            infantry_points += smithy_infantry * quantity;
            cavalry_points += smithy_cavalry * quantity;
        }
        (infantry_points, cavalry_points)
    }

    pub fn scouting_attack_points(&self) -> u64 {
        self.scouting_points(35)
    }

    pub fn scouting_defense_points(&self) -> u64 {
        self.scouting_points(20)
    }

    fn scouting_points(&self, base_points: u8) -> u64 {
        let idx: usize = 3;
        let quantity = self.units[idx];
        let unit = self.get_unit(idx as u8).unwrap();
        let smithy_improvement = self.apply_smithy_upgrade(unit.clone(), idx, base_points as u64);
        smithy_improvement * quantity
    }

    pub fn apply_losses(&mut self, percent: f64) {
        for (idx, quantity) in self.units.into_iter().enumerate() {
            self.units[idx] = quantity - ((quantity as f64) * percent / 100.0).floor() as u64;
        }
    }

    fn apply_smithy_upgrade(&self, unit: Unit, idx: usize, combat_value: u64) -> u64 {
        let level: i32 = self.smithy[idx as usize].into();
        ((combat_value as f64)
            + ((combat_value + 300 * unit.cost.upkeep) as f64 / 7.0)
                * ((1.007f64).powi(level.try_into().unwrap()) - 1.0).floor()) as u64
    }
}

#[derive(Debug, Clone)]
pub struct Unit {
    pub name: UnitName,
    pub role: UnitRole,
    pub attack: u64,
    pub defense_infantry: u64,
    pub defense_cavalry: u64,
    pub speed: u8,
    pub capacity: u32,
    pub cost: Cost,
}

static ROMAN_UNITS: TribeUnits = [
    Unit {
        name: UnitName::Legionnaire,
        role: UnitRole::Infantry,
        attack: 40,
        defense_infantry: 35,
        defense_cavalry: 50,
        speed: 12,
        capacity: 50,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 120,
                clay: 100,
                iron: 150,
                crop: 30,
            },
            upkeep: 1,
            build_time: 533,
        },
    },
    Unit {
        name: UnitName::Praetorian,
        role: UnitRole::Infantry,
        attack: 30,
        defense_infantry: 65,
        defense_cavalry: 35,
        speed: 10,
        capacity: 20,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 100,
                clay: 130,
                iron: 160,
                crop: 70,
            },
            upkeep: 1,
            build_time: 597,
        },
    },
    Unit {
        name: UnitName::Imperian,
        role: UnitRole::Infantry,
        attack: 70,
        defense_infantry: 40,
        defense_cavalry: 25,
        speed: 14,
        capacity: 50,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 150,
                clay: 160,
                iron: 210,
                crop: 80,
            },
            upkeep: 1,
            build_time: 640,
        },
    },
    Unit {
        name: UnitName::EquitesLegati,
        role: UnitRole::Cavalry,
        attack: 0,
        defense_infantry: 20,
        defense_cavalry: 10,
        speed: 32,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 140,
                clay: 160,
                iron: 20,
                crop: 40,
            },
            upkeep: 2,
            build_time: 453,
        },
    },
    Unit {
        name: UnitName::EquitesImperatoris,
        role: UnitRole::Cavalry,
        attack: 120,
        defense_infantry: 65,
        defense_cavalry: 50,
        speed: 28,
        capacity: 100,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 550,
                clay: 440,
                iron: 320,
                crop: 100,
            },
            upkeep: 3,
            build_time: 880,
        },
    },
    Unit {
        name: UnitName::EquitesCaesaris,
        role: UnitRole::Cavalry,
        attack: 180,
        defense_infantry: 80,
        defense_cavalry: 105,
        speed: 20,
        capacity: 70,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 550,
                clay: 640,
                iron: 800,
                crop: 180,
            },
            upkeep: 4,
            build_time: 1173,
        },
    },
    Unit {
        name: UnitName::BatteringRam,
        role: UnitRole::Ram,
        attack: 60,
        defense_infantry: 30,
        defense_cavalry: 75,
        speed: 8,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 900,
                clay: 360,
                iron: 500,
                crop: 70,
            },
            upkeep: 3,
            build_time: 1533,
        },
    },
    Unit {
        name: UnitName::FireCatapult,
        role: UnitRole::Cata,
        attack: 75,
        defense_infantry: 60,
        defense_cavalry: 10,
        speed: 6,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 950,
                clay: 1350,
                iron: 600,
                crop: 90,
            },
            upkeep: 6,
            build_time: 3000,
        },
    },
    Unit {
        name: UnitName::Senator,
        role: UnitRole::Chief,
        attack: 50,
        defense_infantry: 40,
        defense_cavalry: 30,
        speed: 8,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 30750,
                clay: 27200,
                iron: 45000,
                crop: 37500,
            },
            upkeep: 5,
            build_time: 30233,
        },
    },
    Unit {
        name: UnitName::Settler,
        role: UnitRole::Settler,
        attack: 0,
        defense_infantry: 80,
        defense_cavalry: 80,
        speed: 10,
        capacity: 3000,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 4600,
                clay: 4200,
                iron: 5800,
                crop: 4400,
            },
            upkeep: 1,
            build_time: 8967,
        },
    },
];

static TEUTON_UNITS: TribeUnits = [
    Unit {
        name: UnitName::Maceman,
        role: UnitRole::Infantry,
        attack: 40,
        defense_infantry: 20,
        defense_cavalry: 5,
        speed: 14,
        capacity: 60,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 95,
                clay: 75,
                iron: 40,
                crop: 40,
            },
            upkeep: 1,
            build_time: 240,
        },
    },
    Unit {
        name: UnitName::Spearman,
        role: UnitRole::Infantry,
        attack: 10,
        defense_infantry: 35,
        defense_cavalry: 60,
        speed: 14,
        capacity: 40,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 145,
                clay: 70,
                iron: 85,
                crop: 40,
            },
            upkeep: 1,
            build_time: 73,
        },
    },
    Unit {
        name: UnitName::Axeman,
        role: UnitRole::Infantry,
        attack: 60,
        defense_infantry: 30,
        defense_cavalry: 30,
        speed: 12,
        capacity: 50,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 130,
                clay: 120,
                iron: 170,
                crop: 70,
            },
            upkeep: 1,
            build_time: 76,
        },
    },
    Unit {
        name: UnitName::Scout,
        role: UnitRole::Infantry,
        attack: 0,
        defense_infantry: 10,
        defense_cavalry: 5,
        speed: 18,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 160,
                clay: 100,
                iron: 50,
                crop: 50,
            },
            upkeep: 1,
            build_time: 73,
        },
    },
    Unit {
        name: UnitName::Paladin,
        role: UnitRole::Infantry,
        attack: 55,
        defense_infantry: 100,
        defense_cavalry: 40,
        speed: 20,
        capacity: 110,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 370,
                clay: 270,
                iron: 290,
                crop: 75,
            },
            upkeep: 2,
            build_time: 800,
        },
    },
    Unit {
        name: UnitName::TeutonicKnight,
        role: UnitRole::Infantry,
        attack: 150,
        defense_infantry: 50,
        defense_cavalry: 75,
        speed: 18,
        capacity: 80,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 450,
                clay: 515,
                iron: 480,
                crop: 80,
            },
            upkeep: 3,
            build_time: 987,
        },
    },
    Unit {
        name: UnitName::Ram,
        role: UnitRole::Ram,
        attack: 65,
        defense_infantry: 30,
        defense_cavalry: 80,
        speed: 8,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 1000,
                clay: 300,
                iron: 350,
                crop: 70,
            },
            upkeep: 3,
            build_time: 1400,
        },
    },
    Unit {
        name: UnitName::Catapult,
        role: UnitRole::Cata,
        attack: 50,
        defense_infantry: 60,
        defense_cavalry: 10,
        speed: 6,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 900,
                clay: 1200,
                iron: 600,
                crop: 60,
            },
            upkeep: 6,
            build_time: 3000,
        },
    },
    Unit {
        name: UnitName::Chief,
        role: UnitRole::Chief,
        attack: 40,
        defense_infantry: 60,
        defense_cavalry: 40,
        speed: 8,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 35500,
                clay: 26600,
                iron: 25000,
                crop: 27200,
            },
            upkeep: 4,
            build_time: 23500,
        },
    },
    Unit {
        name: UnitName::Settler,
        role: UnitRole::Settler,
        attack: 10,
        defense_infantry: 80,
        defense_cavalry: 80,
        speed: 10,
        capacity: 3000,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 5800,
                clay: 4400,
                iron: 4600,
                crop: 5200,
            },
            upkeep: 1,
            build_time: 10333,
        },
    },
];

static GAUL_UNITS: TribeUnits = [
    Unit {
        name: UnitName::Phalanx,
        role: UnitRole::Infantry,
        attack: 15,
        defense_infantry: 40,
        defense_cavalry: 50,
        speed: 14,
        capacity: 35,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 100,
                clay: 130,
                iron: 55,
                crop: 30,
            },
            upkeep: 1,
            build_time: 347,
        },
    },
    Unit {
        name: UnitName::Swordsman,
        role: UnitRole::Infantry,
        attack: 65,
        defense_infantry: 35,
        defense_cavalry: 20,
        speed: 12,
        capacity: 45,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 140,
                clay: 150,
                iron: 185,
                crop: 60,
            },
            upkeep: 1,
            build_time: 480,
        },
    },
    Unit {
        name: UnitName::Pathfinder,
        role: UnitRole::Cavalry,
        attack: 0,
        defense_infantry: 20,
        defense_cavalry: 10,
        speed: 34,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 170,
                clay: 150,
                iron: 20,
                crop: 40,
            },
            upkeep: 2,
            build_time: 75,
        },
    },
    Unit {
        name: UnitName::TheutatesThunder,
        role: UnitRole::Cavalry,
        attack: 100,
        defense_infantry: 25,
        defense_cavalry: 40,
        speed: 38,
        capacity: 75,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 350,
                clay: 450,
                iron: 230,
                crop: 60,
            },
            upkeep: 2,
            build_time: 827,
        },
    },
    Unit {
        name: UnitName::Druidrider,
        role: UnitRole::Cavalry,
        attack: 45,
        defense_infantry: 115,
        defense_cavalry: 55,
        speed: 32,
        capacity: 35,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 360,
                clay: 330,
                iron: 280,
                crop: 120,
            },
            upkeep: 2,
            build_time: 853,
        },
    },
    Unit {
        name: UnitName::Haeduan,
        role: UnitRole::Cavalry,
        attack: 140,
        defense_infantry: 60,
        defense_cavalry: 165,
        speed: 26,
        capacity: 65,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 500,
                clay: 620,
                iron: 675,
                crop: 170,
            },
            upkeep: 3,
            build_time: 1040,
        },
    },
    Unit {
        name: UnitName::Ram,
        role: UnitRole::Ram,
        attack: 50,
        defense_infantry: 30,
        defense_cavalry: 105,
        speed: 8,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 950,
                clay: 555,
                iron: 330,
                crop: 75,
            },
            upkeep: 3,
            build_time: 1667,
        },
    },
    Unit {
        name: UnitName::Trebuchet,
        role: UnitRole::Cata,
        attack: 70,
        defense_infantry: 45,
        defense_cavalry: 10,
        speed: 6,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 960,
                clay: 1450,
                iron: 630,
                crop: 90,
            },
            upkeep: 6,
            build_time: 3000,
        },
    },
    Unit {
        name: UnitName::Chieftain,
        role: UnitRole::Chief,
        attack: 40,
        defense_infantry: 50,
        defense_cavalry: 50,
        speed: 10,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 30750,
                clay: 45400,
                iron: 31000,
                crop: 37500,
            },
            upkeep: 4,
            build_time: 30233,
        },
    },
    Unit {
        name: UnitName::Settler,
        role: UnitRole::Settler,
        attack: 0,
        defense_infantry: 80,
        defense_cavalry: 80,
        speed: 10,
        capacity: 3000,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 4400,
                clay: 5600,
                iron: 4200,
                crop: 3900,
            },
            upkeep: 1,
            build_time: 7567,
        },
    },
];

static NATURE_UNITS: TribeUnits = [
    Unit {
        name: UnitName::Rat,
        role: UnitRole::Infantry,
        attack: 10,
        defense_infantry: 25,
        defense_cavalry: 20,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 100,
                crop: 100,
            },
            upkeep: 1,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Spider,
        role: UnitRole::Infantry,
        attack: 20,
        defense_infantry: 35,
        defense_cavalry: 40,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 1,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Serpent,
        role: UnitRole::Infantry,
        attack: 60,
        defense_infantry: 40,
        defense_cavalry: 60,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 1,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Bat,
        role: UnitRole::Infantry,
        attack: 80,
        defense_infantry: 66,
        defense_cavalry: 50,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 1,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::WildBoar,
        role: UnitRole::Infantry,
        attack: 50,
        defense_infantry: 70,
        defense_cavalry: 33,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 2,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Wolf,
        role: UnitRole::Infantry,
        attack: 100,
        defense_infantry: 80,
        defense_cavalry: 70,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 2,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Bear,
        role: UnitRole::Infantry,
        attack: 250,
        defense_infantry: 140,
        defense_cavalry: 200,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 3,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Crocodile,
        role: UnitRole::Infantry,
        attack: 450,
        defense_infantry: 380,
        defense_cavalry: 240,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 3,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Tiger,
        role: UnitRole::Infantry,
        attack: 200,
        defense_infantry: 170,
        defense_cavalry: 250,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 3,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Elephant,
        role: UnitRole::Infantry,
        attack: 600,
        defense_infantry: 440,
        defense_cavalry: 520,
        speed: 40,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 5,
            build_time: 0,
        },
    },
];

static NATAR_UNITS: TribeUnits = [
    Unit {
        name: UnitName::Pikeman,
        role: UnitRole::Infantry,
        attack: 20,
        defense_infantry: 35,
        defense_cavalry: 50,
        speed: 12,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 1,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::ThornedWarrior,
        role: UnitRole::Infantry,
        attack: 65,
        defense_infantry: 30,
        defense_cavalry: 10,
        speed: 14,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 1,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Guardsman,
        role: UnitRole::Infantry,
        attack: 100,
        defense_infantry: 90,
        defense_cavalry: 75,
        speed: 12,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 1,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::BirdsOfPrey,
        role: UnitRole::Scout,
        attack: 0,
        defense_infantry: 10,
        defense_cavalry: 0,
        speed: 50,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 1,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Axerider,
        role: UnitRole::Cavalry,
        attack: 155,
        defense_infantry: 80,
        defense_cavalry: 50,
        speed: 28,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 2,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::NatarianKnight,
        role: UnitRole::Cavalry,
        attack: 170,
        defense_infantry: 140,
        defense_cavalry: 80,
        speed: 24,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 3,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Warelephant,
        role: UnitRole::Ram,
        attack: 250,
        defense_infantry: 120,
        defense_cavalry: 150,
        speed: 10,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 4,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Ballista,
        role: UnitRole::Cata,
        attack: 60,
        defense_infantry: 45,
        defense_cavalry: 10,
        speed: 6,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 5,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::NatarianEmperor,
        role: UnitRole::Chief,
        attack: 80,
        defense_infantry: 50,
        defense_cavalry: 50,
        speed: 10,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 1,
            build_time: 0,
        },
    },
    Unit {
        name: UnitName::Settler,
        role: UnitRole::Settler,
        attack: 30,
        defense_infantry: 40,
        defense_cavalry: 40,
        speed: 10,
        capacity: 0,
        cost: Cost {
            resources: ResourceGroup {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            upkeep: 1,
            build_time: 0,
        },
    },
];

fn get_tribe_units(tribe: Tribe) -> TribeUnits {
    match tribe {
        Tribe::Roman => ROMAN_UNITS.clone(),
        Tribe::Gaul => GAUL_UNITS.clone(),
        Tribe::Teuton => TEUTON_UNITS.clone(),
        Tribe::Nature => NATURE_UNITS.clone(),
        Tribe::Natar => NATAR_UNITS.clone(),
    }
}
