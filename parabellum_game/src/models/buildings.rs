use serde::{Deserialize, Serialize};

use parabellum_core::GameError;
use parabellum_types::{
    buildings::{BuildingGroup, BuildingName},
    common::{Cost, ResourceGroup},
    tribe::Tribe,
};

use crate::models::village::VillageBuilding;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Building {
    pub name: BuildingName,
    pub group: BuildingGroup,
    pub value: u32,
    pub population: u32,
    pub culture_points: u16,
    pub level: u8,
}

impl Building {
    pub fn new(name: BuildingName, server_speed: i8) -> Self {
        let building = get_building_data(&name).unwrap();
        let data = building.data[0].clone();
        let level = 1;
        let effective_value = Self::effective_value(&name, data.6, server_speed);
        let (population, culture_points) = get_cumulative_stats(&name, &building.group, level);

        Self {
            name,
            group: building.group,
            culture_points,
            level,
            population,
            value: effective_value,
        }
    }

    /// Returns a new building at the specified level.
    pub fn at_level(&self, mut level: u8, server_speed: i8) -> Result<Self, GameError> {
        let building = get_building_data(&self.name)?;

        if level > building.rules.max_level {
            level = building.rules.max_level
        }

        // max level reached?
        if level >= self.level && self.level == building.rules.max_level {
            return Err(GameError::BuildingMaxLevelReached);
        }

        // resource fields have production values at level 0 too
        let mut lvl_idx = level;
        if level > 0 && self.group != BuildingGroup::Resources {
            lvl_idx -= 1;
        }

        // non-resource fields at level 0 should fallback to level 1
        if level == 0 && self.group != BuildingGroup::Resources {
            level = 1;
        }

        let data = building.data[lvl_idx as usize].clone();
        let effective_value = Self::effective_value(&self.name, data.6, server_speed);
        let (population, culture_points) = get_cumulative_stats(&self.name, &self.group, level);

        Ok(Self {
            name: self.name.clone(),
            group: building.group.clone(),
            culture_points,
            level,
            population,
            value: effective_value,
        })
    }

    pub fn validate_build(
        &self,
        tribe: &Tribe,
        village_buildings: &Vec<VillageBuilding>,
        is_capital: bool,
    ) -> Result<(), GameError> {
        let data = get_building_data(&self.name).unwrap();

        // tribe constraints
        if !data.rules.tribes.is_empty() {
            let ok = data.rules.tribes.contains(tribe);
            if !ok {
                return Err(GameError::BuildingTribeMismatch {
                    building: self.name.clone(),
                    tribe: tribe.clone(),
                });
            }
        }

        // capital/non-capital constraints
        if is_capital
            && data
                .rules
                .constraints
                .contains(&BuildingConstraint::NonCapital)
        {
            return Err(GameError::NonCapitalConstraint(self.name.clone()));
        }

        if !is_capital
            && data
                .rules
                .constraints
                .contains(&BuildingConstraint::OnlyCapital)
        {
            return Err(GameError::CapitalConstraint(self.name.clone()));
        }

        // building requirements (aka technology tree)
        for req in data.rules.requirements {
            match village_buildings
                .iter()
                .find(|&vb| vb.building.name == req.0 && vb.building.level >= req.1)
            {
                Some(_) => (),
                None => {
                    return Err(GameError::BuildingRequirementsNotMet {
                        building: req.0.clone(),
                        level: req.1,
                    });
                }
            };
        }

        for vb in village_buildings {
            // check if a building has conflicts with other buildings (eg: Palace vs Residence)
            for conflict in data.rules.conflicts {
                if vb.building.name == conflict.0 {
                    return Err(GameError::BuildingConflict(
                        self.name.clone(),
                        conflict.0.clone(),
                    ));
                }
            }

            // rules for duplicated buildings (eg: Warehouse or Granary)
            if self.name == vb.building.name {
                // and allows multiple
                if !data.rules.allow_multiple {
                    return Err(GameError::NoMultipleBuildingConstraint(self.name.clone()));
                }
                // and has reached max level
                if self.level != data.rules.max_level {
                    return Err(GameError::MultipleBuildingMaxNotReached(self.name.clone()));
                }
            }
        }

        Ok(())
    }

    pub fn calculate_build_time_secs(&self, server_speed: &i8, mb_level: &u8) -> u32 {
        let base_time = self.cost().time as f64;

        let mb_factor = if *mb_level == 0 {
            1000
        } else {
            let bdata = get_building_data(&BuildingName::MainBuilding).unwrap();
            let level_idx = (*mb_level - 1) as usize;

            bdata
                .data
                .get(level_idx)
                .map_or(bdata.data.last().unwrap().6, |data| data.6)
        };

        let effective_factor: f64 = if self.name == BuildingName::MainBuilding {
            1.0
        } else {
            mb_factor as f64 / 1000.0
        };
        println!("===== building {:#?}======", self);
        println!("===== factor {:#?}======", effective_factor);
        println!("===== speed {:#?}======", server_speed);
        println!("===== base_time {:#?}======", base_time);

        let final_time = (base_time * effective_factor) / (*server_speed) as f64;
        let res = final_time.floor().max(1.0) as u32;
        println!("===== res {:#?}======", res);
        res
    }

    /// Returns the cost of the building.
    pub fn cost(&self) -> Cost {
        let building = get_building_data(&self.name).unwrap();

        let mut level = self.level;

        if level > 0 && self.group != BuildingGroup::Resources {
            level -= 1;
        }

        let data = building.data[level as usize].clone();

        Cost {
            resources: ResourceGroup::new(data.0, data.1, data.2, data.3),
            upkeep: data.4,
            time: data.7,
        }
    }

    /// Returns the building effective value (production/capacity) based on server speed.
    pub fn effective_value(name: &BuildingName, base_value: u32, server_speed: i8) -> u32 {
        if server_speed <= 1 {
            return base_value;
        }

        let speed_multiplier = server_speed as u32;

        match name {
            BuildingName::Woodcutter
            | BuildingName::ClayPit
            | BuildingName::IronMine
            | BuildingName::Cropland => base_value * speed_multiplier,
            BuildingName::Warehouse
            | BuildingName::Granary
            | BuildingName::GreatWarehouse
            | BuildingName::GreatGranary => base_value * speed_multiplier,

            _ => base_value,
        }
    }
}

// lumber, clay, iron, crop, upkeep, culture_points, value, time
#[derive(Debug, Clone)]
#[allow(dead_code)]
/// (lumber, clay, iron, crop, upkeep, culture_points, value, time)
struct BuildingValueData(u32, u32, u32, u32, u32, u16, u32, u32);

#[derive(Debug, Clone, Eq, PartialEq)]
enum BuildingConstraint {
    OnlyCapital,
    NonCapital,
}

#[derive(Debug, Clone)]
struct BuildingRequirement(BuildingName, u8);

#[derive(Debug, Clone)]
struct BuildingConflict(BuildingName);

#[derive(Debug, Clone)]
struct BuildingRules {
    requirements: &'static [BuildingRequirement],
    conflicts: &'static [BuildingConflict],
    tribes: &'static [Tribe],
    max_level: u8,
    constraints: &'static [BuildingConstraint],
    allow_multiple: bool,
}

#[derive(Debug, Clone)]
struct BuildingData {
    data: &'static [BuildingValueData],
    group: BuildingGroup,
    rules: BuildingRules,
}

/// Returns cumulative population and culture points for a given level.
fn get_cumulative_stats(name: &BuildingName, group: &BuildingGroup, level: u8) -> (u32, u16) {
    if level == 0 {
        return (0, 0); // Level 0 has 0 pop and 0 CP
    }

    let building_data = get_building_data(&name).unwrap();
    let mut cumulative_pop = 0;
    let mut cumulative_cp = 0;

    // iterate from level 1 up to current level
    for i in 1..=level {
        let data_idx = if *group == BuildingGroup::Resources {
            i as usize
        } else {
            (i - 1) as usize
        };

        if let Some(data) = building_data.data.get(data_idx) {
            cumulative_pop += data.4;
            cumulative_cp += data.5;
        }
    }
    (cumulative_pop, cumulative_cp)
}

fn get_building_data(name: &BuildingName) -> Result<BuildingData, GameError> {
    match name {
        BuildingName::Woodcutter => Ok(WOODCUTTER.clone()),
        BuildingName::ClayPit => Ok(CLAY_PIT.clone()),
        BuildingName::IronMine => Ok(IRON_MINE.clone()),
        BuildingName::Cropland => Ok(CROPLAND.clone()),
        BuildingName::Sawmill => Ok(SAWMILL.clone()),
        BuildingName::Brickyard => Ok(BRICKYARD.clone()),
        BuildingName::IronFoundry => Ok(IRON_FOUNDRY.clone()),
        BuildingName::GrainMill => Ok(GRAIN_MILL.clone()),
        BuildingName::Bakery => Ok(BAKERY.clone()),
        BuildingName::Warehouse => Ok(WAREHOUSE.clone()),
        BuildingName::Granary => Ok(GRANARY.clone()),
        BuildingName::Smithy => Ok(SMITHY.clone()),
        BuildingName::MainBuilding => Ok(MAIN_BUILDING.clone()),
        BuildingName::RallyPoint => Ok(RALLY_POINT.clone()),
        BuildingName::TournamentSquare => Ok(TOURNAMENT_SQUARE.clone()),
        BuildingName::Marketplace => Ok(MARKETPLACE.clone()),
        BuildingName::Embassy => Ok(EMBASSY.clone()),
        BuildingName::Barracks => Ok(BARRACKS.clone()),
        BuildingName::Stable => Ok(STABLE.clone()),
        BuildingName::Workshop => Ok(WORKSHOP.clone()),
        BuildingName::Academy => Ok(ACADEMY.clone()),
        BuildingName::Cranny => Ok(CRANNY.clone()),
        BuildingName::TownHall => Ok(TOWN_HALL.clone()),
        BuildingName::Residence => Ok(RESIDENCE.clone()),
        BuildingName::Palace => Ok(PALACE.clone()),
        BuildingName::Treasury => Ok(TREASURY.clone()),
        BuildingName::TradeOffice => Ok(TRADE_OFFICE.clone()),
        BuildingName::GreatBarracks => Ok(GREAT_BARRACKS.clone()),
        BuildingName::GreatStable => Ok(GREAT_STABLE.clone()),
        BuildingName::Palisade => Ok(PALISADE.clone()),
        BuildingName::EarthWall => Ok(EARTH_WALL.clone()),
        BuildingName::CityWall => Ok(CITY_WALL.clone()),
        BuildingName::Brewery => Ok(BREWERY.clone()),
        BuildingName::StonemansionLodge => Ok(STONEMANSION_LODGE.clone()),
        BuildingName::Trapper => Ok(TRAPPER.clone()),
        BuildingName::HeroMansion => Ok(HERO_MANSION.clone()),
        BuildingName::GreatWorkshop => Ok(GREAT_WORKSHOP.clone()),
        BuildingName::GreatGranary => Ok(GREAT_GRANARY.clone()),
        BuildingName::GreatWarehouse => Ok(GREAT_WAREHOUSE.clone()),
        BuildingName::HorseDrinkingTrough => Ok(HORSE_DRINKING_TROUGH.clone()),
        BuildingName::WonderOfTheWorld => Ok(WONDER_OF_THW_WORLD.clone()),
        // FIXME: artifacts and construction plans deserve another category
        BuildingName::AncientConstructionPlan => Ok(WONDER_OF_THW_WORLD.clone()),
    }
}

// ==================== BEGIN BUILDINGS STATIC DATA ====================

static WOODCUTTER: BuildingData = BuildingData {
    data: &[
        BuildingValueData(0, 0, 0, 0, 0, 0, 2, 0),
        BuildingValueData(40, 100, 50, 60, 2, 1, 5, 260),
        BuildingValueData(65, 165, 85, 100, 1, 1, 9, 620),
        BuildingValueData(110, 280, 140, 165, 1, 2, 15, 1190),
        BuildingValueData(185, 465, 235, 280, 1, 2, 22, 2100),
        BuildingValueData(310, 780, 390, 465, 1, 2, 33, 3560),
        BuildingValueData(520, 1300, 650, 780, 2, 3, 50, 5890),
        BuildingValueData(870, 2170, 1085, 1300, 2, 4, 70, 9620),
        BuildingValueData(1450, 3625, 1810, 2175, 2, 4, 100, 15590),
        BuildingValueData(2420, 6050, 3025, 3630, 2, 5, 145, 25150),
        BuildingValueData(4040, 10105, 5050, 6060, 2, 6, 200, 40440),
        BuildingValueData(6750, 16870, 8435, 10125, 2, 7, 280, 64900),
        BuildingValueData(11270, 28175, 14090, 16905, 2, 9, 375, 104050),
        BuildingValueData(18820, 47055, 23525, 28230, 2, 11, 495, 166680),
        BuildingValueData(31430, 78580, 39290, 47150, 2, 13, 635, 266880),
        BuildingValueData(52490, 131230, 65615, 78740, 2, 15, 800, 427210),
        BuildingValueData(87660, 219155, 109575, 131490, 3, 18, 1000, 683730),
        BuildingValueData(146395, 365985, 182995, 219590, 3, 22, 1300, 1094170),
        BuildingValueData(244480, 611195, 305600, 366715, 3, 27, 1600, 1750880),
        BuildingValueData(408280, 1020695, 510350, 612420, 3, 32, 2000, 2801600),
        BuildingValueData(681825, 1704565, 852280, 1022740, 3, 38, 2450, 4482770),
    ],
    group: BuildingGroup::Resources,
    rules: BuildingRules {
        requirements: &[],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: true,
    },
};

static CLAY_PIT: BuildingData = BuildingData {
    data: &[
        BuildingValueData(0, 0, 0, 0, 0, 0, 2, 0),
        BuildingValueData(80, 40, 80, 50, 2, 1, 5, 220),
        BuildingValueData(135, 65, 135, 85, 1, 1, 9, 550),
        BuildingValueData(225, 110, 225, 140, 1, 2, 15, 1080),
        BuildingValueData(375, 185, 375, 235, 1, 2, 22, 1930),
        BuildingValueData(620, 310, 620, 390, 1, 2, 33, 3290),
        BuildingValueData(1040, 520, 1040, 650, 2, 3, 50, 5470),
        BuildingValueData(1735, 870, 1735, 1085, 2, 4, 70, 8950),
        BuildingValueData(2900, 1450, 2900, 1810, 2, 4, 100, 14520),
        BuildingValueData(4840, 2420, 4840, 3025, 2, 5, 145, 23430),
        BuildingValueData(8080, 4040, 8080, 5050, 2, 6, 200, 37690),
        BuildingValueData(13500, 6750, 13500, 8435, 2, 7, 280, 60510),
        BuildingValueData(22540, 11270, 22540, 14090, 2, 9, 375, 97010),
        BuildingValueData(37645, 18820, 37645, 23525, 2, 11, 11495, 155420),
        BuildingValueData(62865, 31430, 62865, 39290, 2, 13, 13635, 248870),
        BuildingValueData(104985, 52490, 104985, 65615, 2, 15, 15800, 398390),
        BuildingValueData(175320, 87660, 175320, 109575, 3, 18, 181000, 637620),
        BuildingValueData(292790, 146395, 292790, 182995, 3, 22, 221300, 1020390),
        BuildingValueData(488955, 244480, 488955, 305600, 3, 27, 271600, 1632820),
        BuildingValueData(816555, 408280, 816555, 510350, 3, 32, 322000, 2612710),
        BuildingValueData(1363650, 681825, 1363650, 852280, 3, 38, 382450, 4180540),
    ],
    group: BuildingGroup::Resources,
    rules: BuildingRules {
        requirements: &[],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: true,
    },
};

static IRON_MINE: BuildingData = BuildingData {
    data: &[
        BuildingValueData(0, 0, 0, 0, 0, 0, 2, 0),
        BuildingValueData(100, 80, 30, 60, 3, 1, 5, 450),
        BuildingValueData(165, 135, 50, 100, 2, 1, 9, 920),
        BuildingValueData(280, 225, 85, 165, 2, 2, 15, 1670),
        BuildingValueData(465, 375, 140, 280, 2, 2, 22, 2880),
        BuildingValueData(780, 620, 235, 465, 2, 2, 33, 4800),
        BuildingValueData(1300, 1040, 390, 780, 2, 3, 50, 7880),
        BuildingValueData(2170, 1735, 650, 1300, 2, 4, 70, 12810),
        BuildingValueData(3625, 2900, 1085, 2175, 2, 4, 100, 20690),
        BuildingValueData(6050, 4840, 1815, 3630, 2, 5, 145, 33310),
        BuildingValueData(10105, 8080, 3030, 6060, 2, 6, 200, 53500),
        BuildingValueData(16870, 13500, 5060, 10125, 3, 7, 280, 85800),
        BuildingValueData(28175, 22540, 8455, 16905, 3, 9, 375, 137470),
        BuildingValueData(47055, 37645, 14115, 28230, 3, 11, 11495, 220160),
        BuildingValueData(78580, 62865, 23575, 47150, 3, 13, 13635, 352450),
        BuildingValueData(131230, 104985, 39370, 78740, 3, 15, 15800, 564120),
        BuildingValueData(219155, 175320, 65745, 131490, 3, 18, 181000, 902760),
        BuildingValueData(365985, 292790, 109795, 219590, 3, 22, 221300, 145546),
        BuildingValueData(611195, 488955, 183360, 366715, 3, 27, 271600, 2311660),
        BuildingValueData(1020695, 816555, 306210, 612420, 3, 32, 322000, 3698850),
        BuildingValueData(1704565, 1363650, 511370, 1022740, 3, 38, 382450, 5918370),
    ],
    group: BuildingGroup::Resources,
    rules: BuildingRules {
        requirements: &[],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: true,
    },
};

static CROPLAND: BuildingData = BuildingData {
    data: &[
        BuildingValueData(0, 0, 0, 0, 0, 0, 2, 0),
        BuildingValueData(70, 90, 70, 20, 0, 1, 5, 150),
        BuildingValueData(115, 150, 115, 35, 0, 1, 9, 440),
        BuildingValueData(195, 250, 195, 55, 0, 2, 15, 900),
        BuildingValueData(325, 420, 325, 95, 0, 2, 22, 1650),
        BuildingValueData(545, 700, 545, 155, 0, 2, 33, 2830),
        BuildingValueData(910, 1170, 910, 260, 1, 3, 50, 4730),
        BuildingValueData(1520, 1950, 1520, 435, 1, 4, 70, 7780),
        BuildingValueData(2535, 3260, 2535, 725, 1, 4, 100, 12190),
        BuildingValueData(4235, 5445, 4235, 1210, 1, 5, 145, 19690),
        BuildingValueData(7070, 9095, 7070, 2020, 1, 6, 200, 31700),
        BuildingValueData(11810, 15185, 11810, 3375, 1, 7, 280, 50910),
        BuildingValueData(19725, 25360, 19725, 5635, 1, 9, 375, 84700),
        BuildingValueData(32940, 42350, 32940, 9410, 1, 11, 495, 135_710),
        BuildingValueData(55005, 70720, 55005, 15715, 1, 13, 635, 217_340),
        BuildingValueData(91860, 118_105, 91860, 26245, 1, 15, 800, 347_950),
        BuildingValueData(153_405, 197_240, 153_405, 43830, 2, 18, 1000, 556_910),
        BuildingValueData(256_190, 329_385, 256_190, 73195, 2, 22, 1300, 891_260),
        BuildingValueData(427_835, 550_075, 427_835, 122_240, 2, 27, 1600, 1_426_210),
        BuildingValueData(714_485, 918_625, 714_485, 204_140, 2, 32, 2000, 2_282_140),
        BuildingValueData(
            1_193_195, 1_534_105, 1_193_195, 340_915, 2, 38, 2450, 3_651_630,
        ),
    ],
    group: BuildingGroup::Resources,
    rules: BuildingRules {
        requirements: &[],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: true,
    },
};

static SAWMILL: BuildingData = BuildingData {
    data: &[
        BuildingValueData(520, 380, 290, 90, 4, 1, 5, 3000),
        BuildingValueData(935, 685, 520, 160, 2, 1, 10, 5700),
        BuildingValueData(1685, 1230, 940, 290, 2, 2, 15, 9750),
        BuildingValueData(3035, 2215, 1690, 525, 2, 2, 20, 15830),
        BuildingValueData(5460, 3990, 3045, 945, 2, 2, 25, 24940),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::Woodcutter, 10),
            BuildingRequirement(BuildingName::MainBuilding, 5),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 5,
        constraints: &[],
        allow_multiple: false,
    },
};

static BRICKYARD: BuildingData = BuildingData {
    data: &[
        BuildingValueData(440, 480, 320, 50, 3, 1, 5, 2240),
        BuildingValueData(790, 865, 575, 90, 2, 1, 10, 4560),
        BuildingValueData(1425, 1555, 1035, 160, 2, 2, 15, 8040),
        BuildingValueData(2565, 2800, 1865, 290, 2, 2, 20, 13260),
        BuildingValueData(4620, 5040, 3360, 525, 2, 2, 25, 21090),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::ClayPit, 10),
            BuildingRequirement(BuildingName::MainBuilding, 5),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 5,
        constraints: &[],
        allow_multiple: false,
    },
};

static IRON_FOUNDRY: BuildingData = BuildingData {
    data: &[
        BuildingValueData(200, 450, 510, 120, 6, 1, 5, 4080),
        BuildingValueData(360, 810, 920, 215, 3, 1, 10, 7320),
        BuildingValueData(650, 1460, 1650, 390, 3, 2, 15, 12180),
        BuildingValueData(1165, 2625, 2975, 700, 3, 2, 20, 19470),
        BuildingValueData(2100, 4725, 5355, 1260, 3, 2, 25, 30410),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::IronMine, 10),
            BuildingRequirement(BuildingName::MainBuilding, 5),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 5,
        constraints: &[],
        allow_multiple: false,
    },
};

static GRAIN_MILL: BuildingData = BuildingData {
    data: &[
        BuildingValueData(500, 440, 380, 1240, 3, 1, 5, 1840),
        BuildingValueData(900, 790, 685, 2230, 2, 1, 10, 3960),
        BuildingValueData(1620, 1425, 1230, 4020, 2, 2, 15, 7140),
        BuildingValueData(2915, 2565, 2215, 7230, 2, 2, 20, 11910),
        BuildingValueData(5250, 4620, 3990, 13015, 2, 2, 25, 19070),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[BuildingRequirement(BuildingName::Cropland, 5)],
        conflicts: &[],
        tribes: &[],
        max_level: 5,
        constraints: &[],
        allow_multiple: false,
    },
};

static BAKERY: BuildingData = BuildingData {
    data: &[
        BuildingValueData(200, 450, 510, 120, 6, 1, 5, 4080),
        BuildingValueData(360, 810, 920, 215, 3, 1, 10, 7320),
        BuildingValueData(650, 1460, 1650, 390, 3, 2, 15, 12180),
        BuildingValueData(1165, 2625, 2975, 700, 3, 2, 20, 19470),
        BuildingValueData(2100, 4725, 5355, 1260, 3, 2, 25, 30410),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::Cropland, 10),
            BuildingRequirement(BuildingName::GrainMill, 5),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 5,
        constraints: &[],
        allow_multiple: false,
    },
};

static WAREHOUSE: BuildingData = BuildingData {
    data: &[
        BuildingValueData(130, 160, 90, 40, 1, 1, 1200, 2000),
        BuildingValueData(165, 205, 115, 50, 1, 1, 1700, 2620),
        BuildingValueData(215, 260, 145, 65, 1, 2, 2300, 3340),
        BuildingValueData(275, 335, 190, 85, 1, 2, 3100, 4170),
        BuildingValueData(350, 430, 240, 105, 1, 2, 4000, 5140),
        BuildingValueData(445, 550, 310, 135, 1, 3, 5000, 6260),
        BuildingValueData(570, 705, 395, 175, 1, 4, 6300, 7570),
        BuildingValueData(730, 900, 505, 225, 1, 4, 7800, 9080),
        BuildingValueData(935, 1115, 650, 290, 1, 5, 9600, 10830),
        BuildingValueData(1200, 1475, 830, 370, 1, 6, 11800, 12860),
        BuildingValueData(1535, 1890, 1065, 470, 2, 7, 14400, 15220),
        BuildingValueData(1965, 2420, 1360, 605, 2, 9, 17600, 17950),
        BuildingValueData(2515, 3095, 1740, 775, 2, 11, 21400, 21130),
        BuildingValueData(3220, 3960, 2230, 990, 2, 13, 25900, 24810),
        BuildingValueData(4120, 5070, 2850, 1270, 2, 15, 31300, 29080),
        BuildingValueData(5275, 6490, 3650, 1625, 2, 18, 37900, 34030),
        BuildingValueData(6750, 8310, 4675, 2075, 2, 22, 45700, 39770),
        BuildingValueData(8640, 10635, 5980, 2660, 2, 27, 55100, 46440),
        BuildingValueData(11060, 13610, 7655, 3405, 2, 32, 66400, 54170),
        BuildingValueData(14155, 17420, 9800, 4355, 2, 38, 80000, 63130),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[BuildingRequirement(BuildingName::MainBuilding, 1)],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: true,
    },
};

static GRANARY: BuildingData = BuildingData {
    data: &[
        BuildingValueData(80, 100, 70, 20, 1, 1, 1200, 1600),
        BuildingValueData(100, 130, 90, 25, 1, 1, 1700, 2160),
        BuildingValueData(130, 165, 115, 35, 1, 2, 2300, 2800),
        BuildingValueData(170, 210, 145, 40, 1, 2, 3100, 3550),
        BuildingValueData(215, 270, 190, 55, 1, 2, 4000, 4420),
        BuildingValueData(275, 345, 240, 70, 1, 3, 5000, 5420),
        BuildingValueData(350, 440, 310, 90, 1, 4, 6300, 6590),
        BuildingValueData(450, 565, 395, 115, 1, 4, 7800, 7950),
        BuildingValueData(575, 720, 505, 145, 1, 5, 9600, 9520),
        BuildingValueData(740, 920, 645, 185, 1, 6, 11800, 11340),
        BuildingValueData(945, 1180, 825, 235, 2, 7, 14400, 13450),
        BuildingValueData(1210, 1510, 1060, 300, 2, 9, 17600, 15910),
        BuildingValueData(1545, 1935, 1355, 385, 2, 11, 21400, 18750),
        BuildingValueData(1980, 2475, 1735, 495, 2, 13, 25900, 22050),
        BuildingValueData(2535, 3170, 2220, 635, 2, 15, 31300, 25880),
        BuildingValueData(3245, 4055, 2840, 810, 2, 18, 37900, 30320),
        BuildingValueData(4155, 5190, 3635, 1040, 2, 22, 45700, 35470),
        BuildingValueData(5315, 6645, 4650, 1330, 2, 27, 55100, 41450),
        BuildingValueData(6805, 8505, 5955, 1700, 2, 32, 66400, 48380),
        BuildingValueData(8710, 10890, 7620, 2180, 2, 38, 80000, 56420),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[BuildingRequirement(BuildingName::MainBuilding, 1)],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: true,
    },
};

static SMITHY: BuildingData = BuildingData {
    data: &[
        BuildingValueData(180, 250, 500, 160, 4, 2, 101, 2000),
        BuildingValueData(230, 320, 640, 205, 2, 3, 102, 2620),
        BuildingValueData(295, 410, 820, 260, 2, 3, 103, 3340),
        BuildingValueData(375, 525, 1050, 335, 2, 4, 104, 4170),
        BuildingValueData(485, 670, 1340, 430, 2, 5, 105, 5140),
        BuildingValueData(620, 860, 1720, 550, 3, 6, 106, 6260),
        BuildingValueData(790, 1100, 2200, 705, 3, 7, 107, 7570),
        BuildingValueData(1015, 1405, 2815, 900, 3, 9, 108, 9080),
        BuildingValueData(1295, 1800, 3605, 1155, 3, 10, 109, 10830),
        BuildingValueData(1660, 2305, 4610, 1475, 3, 12, 110, 12860),
        BuildingValueData(2125, 2950, 5905, 1890, 3, 15, 111, 15220),
        BuildingValueData(2720, 3780, 7555, 2420, 3, 18, 112, 17950),
        BuildingValueData(3480, 4835, 9670, 3095, 3, 21, 113, 21130),
        BuildingValueData(4455, 6190, 12380, 3960, 3, 26, 114, 24810),
        BuildingValueData(5705, 7925, 15845, 5070, 3, 31, 115, 29080),
        BuildingValueData(7300, 10140, 20280, 6490, 4, 37, 116, 34030),
        BuildingValueData(9345, 12980, 25960, 8310, 4, 44, 117, 39770),
        BuildingValueData(11965, 16615, 33230, 10635, 4, 53, 118, 46440),
        BuildingValueData(15315, 21270, 42535, 13610, 4, 64, 119, 54170),
        BuildingValueData(19600, 27225, 54445, 17420, 4, 77, 120, 63130),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::MainBuilding, 3),
            BuildingRequirement(BuildingName::Academy, 3),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static TOURNAMENT_SQUARE: BuildingData = BuildingData {
    data: &[
        BuildingValueData(1750, 2250, 1530, 240, 1, 1, 110, 3500),
        BuildingValueData(2240, 2880, 1960, 305, 1, 1, 120, 4360),
        BuildingValueData(2865, 3685, 2505, 395, 1, 2, 130, 5360),
        BuildingValueData(3670, 4720, 3210, 505, 1, 2, 140, 6510),
        BuildingValueData(4700, 6040, 4105, 645, 1, 2, 150, 7860),
        BuildingValueData(6015, 7730, 5255, 825, 1, 3, 160, 9410),
        BuildingValueData(7695, 9895, 6730, 1055, 1, 4, 170, 11220),
        BuildingValueData(9850, 12665, 8615, 1350, 1, 4, 180, 13320),
        BuildingValueData(12610, 16215, 11025, 1730, 1, 5, 190, 15750),
        BuildingValueData(16140, 20755, 14110, 2215, 1, 6, 200, 18570),
        BuildingValueData(20660, 26565, 18065, 2835, 2, 7, 210, 21840),
        BuildingValueData(26445, 34000, 23120, 3625, 2, 9, 220, 25630),
        BuildingValueData(33850, 43520, 29595, 4640, 2, 11, 230, 30030),
        BuildingValueData(43330, 55705, 37880, 5940, 2, 13, 240, 35140),
        BuildingValueData(55460, 71305, 48490, 7605, 2, 15, 250, 41060),
        BuildingValueData(70990, 91270, 62065, 9735, 2, 18, 260, 47930),
        BuildingValueData(90865, 116825, 79440, 12460, 2, 22, 270, 55900),
        BuildingValueData(116305, 149540, 101685, 15950, 2, 27, 280, 65140),
        BuildingValueData(148875, 191410, 130160, 20415, 2, 32, 290, 75860),
        BuildingValueData(190560, 245005, 166600, 26135, 2, 38, 300, 88300),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[BuildingRequirement(BuildingName::RallyPoint, 15)],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static MAIN_BUILDING: BuildingData = BuildingData {
    data: &[
        BuildingValueData(70, 40, 60, 20, 2, 2, 1000, 2620),
        BuildingValueData(90, 50, 75, 25, 1, 3, 964, 3220),
        BuildingValueData(115, 65, 100, 35, 1, 3, 929, 3880),
        BuildingValueData(145, 85, 125, 40, 1, 4, 896, 4610),
        BuildingValueData(190, 105, 160, 55, 1, 5, 864, 5410),
        BuildingValueData(240, 135, 205, 70, 2, 6, 833, 6300),
        BuildingValueData(310, 175, 265, 90, 2, 7, 803, 7280),
        BuildingValueData(395, 225, 340, 115, 2, 9, 774, 8380),
        BuildingValueData(505, 290, 430, 145, 2, 10, 746, 9590),
        BuildingValueData(645, 370, 555, 185, 2, 12, 719, 10940),
        BuildingValueData(825, 470, 710, 235, 2, 15, 693, 12440),
        BuildingValueData(1060, 605, 905, 300, 2, 18, 668, 14120),
        BuildingValueData(1355, 775, 1160, 385, 2, 21, 644, 15980),
        BuildingValueData(1735, 990, 1485, 495, 2, 26, 621, 18050),
        BuildingValueData(2220, 1270, 1900, 635, 2, 31, 599, 20370),
        BuildingValueData(2840, 1625, 2435, 810, 3, 37, 577, 22950),
        BuildingValueData(3635, 2075, 3115, 1040, 3, 44, 556, 25830),
        BuildingValueData(4650, 2660, 3990, 1330, 3, 53, 536, 29040),
        BuildingValueData(5955, 3405, 5105, 1700, 3, 64, 517, 32630),
        BuildingValueData(7620, 4355, 6535, 2180, 3, 77, 498, 32632),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static RALLY_POINT: BuildingData = BuildingData {
    data: &[
        BuildingValueData(70, 40, 60, 20, 2, 2, 0, 2620),
        BuildingValueData(90, 50, 75, 25, 1, 3, 0, 3220),
        BuildingValueData(115, 65, 100, 35, 1, 3, 0, 3880),
        BuildingValueData(145, 85, 125, 40, 1, 4, 0, 4610),
        BuildingValueData(190, 105, 160, 55, 1, 5, 0, 5410),
        BuildingValueData(240, 135, 205, 70, 2, 6, 0, 6300),
        BuildingValueData(310, 175, 265, 90, 2, 7, 0, 7280),
        BuildingValueData(395, 225, 340, 115, 2, 9, 0, 8380),
        BuildingValueData(505, 290, 430, 145, 2, 10, 0, 9590),
        BuildingValueData(645, 370, 555, 185, 2, 12, 0, 10940),
        BuildingValueData(825, 470, 710, 235, 2, 15, 0, 12440),
        BuildingValueData(1060, 605, 905, 300, 2, 18, 0, 14120),
        BuildingValueData(1355, 775, 1160, 385, 2, 21, 0, 15980),
        BuildingValueData(1735, 990, 1485, 495, 2, 26, 0, 18050),
        BuildingValueData(2220, 1270, 1900, 635, 2, 31, 0, 20370),
        BuildingValueData(2840, 1625, 2435, 810, 3, 37, 0, 22950),
        BuildingValueData(3635, 2075, 3115, 1040, 3, 44, 0, 25830),
        BuildingValueData(4650, 2660, 3990, 1330, 3, 53, 0, 29040),
        BuildingValueData(5955, 3405, 5105, 1700, 3, 64, 0, 32630),
        BuildingValueData(7620, 4355, 6535, 2180, 3, 77, 0, 32632),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static MARKETPLACE: BuildingData = BuildingData {
    data: &[
        BuildingValueData(80, 70, 120, 70, 4, 4, 1, 1800),
        BuildingValueData(100, 90, 155, 90, 2, 4, 2, 2390),
        BuildingValueData(130, 115, 195, 115, 2, 5, 3, 3070),
        BuildingValueData(170, 145, 250, 145, 2, 6, 4, 3860),
        BuildingValueData(215, 190, 320, 190, 2, 7, 5, 4780),
        BuildingValueData(275, 240, 410, 240, 3, 9, 6, 5840),
        BuildingValueData(350, 310, 530, 310, 3, 11, 7, 7080),
        BuildingValueData(450, 395, 675, 395, 3, 13, 8, 8510),
        BuildingValueData(575, 505, 865, 505, 3, 15, 9, 10170),
        BuildingValueData(740, 645, 1105, 645, 3, 19, 10, 12100),
        BuildingValueData(945, 825, 1415, 825, 3, 22, 11, 14340),
        BuildingValueData(1210, 1060, 1815, 1060, 3, 27, 12, 16930),
        BuildingValueData(1545, 1355, 2320, 1355, 3, 32, 13, 19940),
        BuildingValueData(1980, 1735, 2970, 1735, 3, 39, 14, 23430),
        BuildingValueData(2535, 2220, 3805, 2220, 3, 46, 15, 27480),
        BuildingValueData(3245, 2840, 4870, 2840, 4, 55, 16, 32180),
        BuildingValueData(4155, 3635, 6230, 3635, 4, 67, 17, 37620),
        BuildingValueData(5315, 4650, 7975, 4650, 4, 80, 18, 43940),
        BuildingValueData(6805, 5955, 10210, 5955, 4, 96, 19, 51270),
        BuildingValueData(8710, 7620, 13065, 7620, 4, 115, 20, 59780),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::MainBuilding, 1),
            BuildingRequirement(BuildingName::Warehouse, 1),
            BuildingRequirement(BuildingName::Granary, 1),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static EMBASSY: BuildingData = BuildingData {
    data: &[
        BuildingValueData(180, 130, 150, 80, 3, 5, 0, 2000),
        BuildingValueData(230, 165, 190, 100, 2, 6, 0, 2620),
        BuildingValueData(295, 215, 245, 130, 2, 7, 9, 3340),
        BuildingValueData(375, 275, 315, 170, 2, 8, 12, 4170),
        BuildingValueData(485, 350, 405, 215, 2, 10, 15, 5140),
        BuildingValueData(620, 445, 515, 275, 2, 12, 18, 6260),
        BuildingValueData(790, 570, 660, 350, 2, 14, 21, 7570),
        BuildingValueData(1015, 730, 845, 450, 2, 17, 24, 9080),
        BuildingValueData(1295, 935, 1080, 575, 2, 21, 27, 10830),
        BuildingValueData(1660, 1200, 1385, 740, 2, 25, 30, 12860),
        BuildingValueData(2125, 1535, 1770, 945, 3, 30, 33, 15220),
        BuildingValueData(2720, 1965, 2265, 1210, 3, 36, 36, 17950),
        BuildingValueData(3480, 2515, 2900, 1545, 3, 43, 39, 21130),
        BuildingValueData(4455, 3220, 3715, 1980, 3, 51, 42, 24810),
        BuildingValueData(5705, 4120, 4755, 2535, 3, 62, 45, 29080),
        BuildingValueData(7300, 5275, 6085, 3245, 3, 74, 48, 34030),
        BuildingValueData(9345, 6750, 7790, 4155, 3, 89, 51, 39770),
        BuildingValueData(11965, 8640, 9970, 5315, 3, 106, 54, 46440),
        BuildingValueData(15315, 11060, 12760, 6805, 3, 128, 57, 54170),
        BuildingValueData(19600, 14155, 16335, 8710, 3, 153, 60, 63130),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[BuildingRequirement(BuildingName::MainBuilding, 1)],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static BARRACKS: BuildingData = BuildingData {
    data: &[
        BuildingValueData(210, 140, 260, 120, 4, 1, 100, 2000),
        BuildingValueData(270, 180, 335, 155, 2, 1, 90, 2620),
        BuildingValueData(345, 230, 425, 195, 2, 2, 81, 3340),
        BuildingValueData(440, 295, 545, 250, 2, 2, 73, 4170),
        BuildingValueData(565, 375, 700, 320, 2, 2, 66, 5140),
        BuildingValueData(720, 480, 895, 410, 3, 3, 59, 6260),
        BuildingValueData(925, 615, 1145, 530, 3, 4, 53, 7570),
        BuildingValueData(1180, 790, 1465, 675, 3, 4, 48, 9080),
        BuildingValueData(1515, 1010, 1875, 865, 3, 5, 43, 10830),
        BuildingValueData(1935, 1290, 2400, 1105, 3, 6, 39, 12860),
        BuildingValueData(2480, 1655, 3070, 1415, 3, 7, 35, 15220),
        BuildingValueData(3175, 2115, 3930, 1815, 3, 9, 31, 17950),
        BuildingValueData(4060, 2710, 5030, 2320, 3, 11, 28, 21130),
        BuildingValueData(5200, 3465, 6435, 2970, 3, 13, 25, 24810),
        BuildingValueData(6655, 4435, 8240, 3805, 3, 15, 23, 29080),
        BuildingValueData(8520, 5680, 10545, 4870, 4, 18, 21, 34030),
        BuildingValueData(10905, 7270, 13500, 6230, 4, 22, 19, 39770),
        BuildingValueData(13955, 9305, 17280, 7975, 4, 27, 17, 46440),
        BuildingValueData(17865, 11910, 22120, 10210, 4, 32, 15, 54170),
        BuildingValueData(22865, 15245, 28310, 13065, 4, 38, 13, 63130),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::MainBuilding, 1),
            BuildingRequirement(BuildingName::RallyPoint, 1),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static STABLE: BuildingData = BuildingData {
    data: &[
        BuildingValueData(260, 140, 220, 100, 5, 2, 100, 2200),
        BuildingValueData(335, 180, 280, 130, 3, 3, 90, 2850),
        BuildingValueData(425, 230, 360, 165, 3, 3, 81, 3610),
        BuildingValueData(545, 295, 460, 210, 3, 4, 73, 4490),
        BuildingValueData(700, 375, 590, 270, 3, 5, 66, 5500),
        BuildingValueData(895, 480, 755, 345, 3, 6, 59, 6680),
        BuildingValueData(1145, 615, 970, 440, 3, 7, 53, 8050),
        BuildingValueData(1465, 790, 1240, 565, 3, 9, 48, 9640),
        BuildingValueData(1875, 1010, 1585, 720, 3, 10, 43, 11480),
        BuildingValueData(2400, 1290, 2030, 920, 3, 12, 39, 13620),
        BuildingValueData(3070, 1655, 2595, 1180, 4, 15, 35, 16100),
        BuildingValueData(3930, 2115, 3325, 1510, 4, 18, 31, 18980),
        BuildingValueData(5030, 2710, 4255, 1935, 4, 21, 28, 22310),
        BuildingValueData(6435, 3465, 5445, 2475, 4, 26, 25, 26180),
        BuildingValueData(8240, 4435, 6970, 3170, 4, 31, 23, 30670),
        BuildingValueData(10545, 5680, 8925, 4055, 4, 37, 21, 35880),
        BuildingValueData(13500, 7270, 11425, 5190, 4, 44, 19, 41920),
        BuildingValueData(17280, 9305, 14620, 6645, 4, 53, 17, 48930),
        BuildingValueData(22120, 11910, 18715, 8505, 4, 64, 15, 57060),
        BuildingValueData(28310, 15245, 23955, 10890, 4, 77, 13, 66490),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::Smithy, 3),
            BuildingRequirement(BuildingName::Academy, 5),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static WORKSHOP: BuildingData = BuildingData {
    data: &[
        BuildingValueData(460, 510, 600, 320, 3, 4, 100, 3000),
        BuildingValueData(590, 655, 770, 410, 2, 4, 90, 3780),
        BuildingValueData(755, 835, 985, 525, 2, 5, 81, 4680),
        BuildingValueData(965, 1070, 1260, 670, 2, 6, 73, 5730),
        BuildingValueData(1235, 1370, 1610, 860, 2, 7, 66, 6950),
        BuildingValueData(1580, 1750, 2060, 1100, 2, 9, 59, 8360),
        BuildingValueData(2025, 2245, 2640, 1405, 2, 11, 53, 10000),
        BuildingValueData(2590, 2870, 3380, 1800, 2, 13, 48, 11900),
        BuildingValueData(3315, 3675, 4325, 2305, 2, 15, 43, 14110),
        BuildingValueData(4245, 4705, 5535, 2950, 2, 19, 39, 16660),
        BuildingValueData(5430, 6020, 7085, 3780, 3, 22, 35, 19630),
        BuildingValueData(6950, 7705, 9065, 4835, 3, 27, 31, 23070),
        BuildingValueData(8900, 9865, 11605, 6190, 3, 32, 28, 27060),
        BuildingValueData(11390, 12625, 14855, 7925, 3, 39, 25, 31690),
        BuildingValueData(14580, 16165, 19015, 10140, 3, 46, 23, 37060),
        BuildingValueData(18660, 20690, 24340, 12980, 3, 55, 21, 43290),
        BuildingValueData(23885, 26480, 31155, 16615, 3, 67, 19, 50520),
        BuildingValueData(30570, 33895, 39875, 21270, 3, 80, 17, 58900),
        BuildingValueData(39130, 43385, 51040, 27225, 3, 96, 15, 68630),
        BuildingValueData(50090, 55535, 65335, 34845, 3, 115, 13, 79910),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::MainBuilding, 5),
            BuildingRequirement(BuildingName::Academy, 10),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static ACADEMY: BuildingData = BuildingData {
    data: &[
        BuildingValueData(220, 160, 90, 40, 4, 5, 1, 2000),
        BuildingValueData(280, 205, 115, 50, 2, 6, 1, 2620),
        BuildingValueData(360, 260, 145, 65, 2, 7, 1, 3340),
        BuildingValueData(460, 335, 190, 85, 2, 8, 1, 4170),
        BuildingValueData(590, 430, 240, 105, 2, 10, 1, 5140),
        BuildingValueData(755, 550, 310, 135, 3, 12, 1, 6260),
        BuildingValueData(970, 705, 395, 175, 3, 14, 1, 7570),
        BuildingValueData(1240, 900, 505, 225, 3, 17, 1, 9080),
        BuildingValueData(1585, 1155, 650, 290, 3, 21, 1, 10830),
        BuildingValueData(2030, 1475, 830, 370, 3, 25, 1, 12860),
        BuildingValueData(2595, 1890, 1065, 470, 3, 30, 1, 15220),
        BuildingValueData(3325, 2420, 1360, 605, 3, 36, 1, 17950),
        BuildingValueData(4255, 3095, 1740, 775, 3, 43, 1, 21130),
        BuildingValueData(5445, 3960, 2230, 990, 3, 51, 1, 24810),
        BuildingValueData(6970, 5070, 2850, 1270, 3, 62, 1, 29080),
        BuildingValueData(8925, 6490, 3650, 1625, 4, 74, 1, 34030),
        BuildingValueData(11425, 8310, 4675, 2075, 4, 89, 1, 39770),
        BuildingValueData(14620, 10635, 5980, 2660, 4, 106, 1, 46440),
        BuildingValueData(18715, 13610, 7655, 3405, 4, 128, 1, 54170),
        BuildingValueData(23955, 17420, 9800, 4355, 4, 153, 1, 63134),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::MainBuilding, 3),
            BuildingRequirement(BuildingName::Barracks, 3),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static CRANNY: BuildingData = BuildingData {
    data: &[
        BuildingValueData(40, 50, 30, 10, 0, 1, 100, 750),
        BuildingValueData(50, 65, 40, 15, 0, 1, 130, 1170),
        BuildingValueData(65, 80, 50, 15, 0, 2, 170, 1660),
        BuildingValueData(85, 105, 65, 20, 0, 2, 220, 2220),
        BuildingValueData(105, 135, 80, 25, 0, 2, 280, 2880),
        BuildingValueData(135, 170, 105, 35, 1, 3, 360, 3640),
        BuildingValueData(175, 220, 130, 45, 1, 4, 460, 4520),
        BuildingValueData(225, 280, 170, 55, 1, 4, 600, 5540),
        BuildingValueData(290, 360, 215, 70, 1, 5, 770, 6730),
        BuildingValueData(370, 460, 275, 90, 1, 6, 1000, 8110),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[],
        conflicts: &[],
        tribes: &[],
        max_level: 10,
        constraints: &[],
        allow_multiple: true,
    },
};

static TOWN_HALL: BuildingData = BuildingData {
    data: &[
        BuildingValueData(1250, 1110, 1260, 600, 4, 6, 100, 12500),
        BuildingValueData(1600, 1420, 1615, 770, 2, 7, 96, 14800),
        BuildingValueData(2050, 1820, 2065, 985, 2, 9, 92, 17468),
        BuildingValueData(2620, 2330, 2640, 1260, 2, 10, 90, 20563),
        BuildingValueData(3355, 2980, 3380, 1610, 2, 12, 88, 24153),
        BuildingValueData(4295, 3815, 4330, 2060, 3, 15, 86, 28317),
        BuildingValueData(5500, 4880, 5540, 2640, 3, 18, 84, 33148),
        BuildingValueData(7035, 6250, 7095, 3380, 3, 21, 82, 38752),
        BuildingValueData(9005, 8000, 9080, 4325, 3, 26, 80, 45252),
        BuildingValueData(11530, 10240, 11620, 5535, 3, 31, 78, 52793),
        BuildingValueData(14755, 13105, 14875, 7085, 3, 37, 76, 61539),
        BuildingValueData(18890, 16775, 19040, 9065, 3, 45, 74, 71686),
        BuildingValueData(24180, 21470, 24370, 11605, 3, 53, 72, 83455),
        BuildingValueData(30950, 27480, 31195, 14855, 3, 64, 70, 97108),
        BuildingValueData(39615, 35175, 39930, 19015, 3, 77, 68, 112946),
        BuildingValueData(50705, 45025, 51110, 24340, 4, 92, 64, 131317),
        BuildingValueData(64905, 57635, 65425, 31155, 4, 111, 62, 152628),
        BuildingValueData(83075, 73770, 83740, 39875, 4, 133, 60, 177348),
        BuildingValueData(106340, 94430, 107190, 51040, 4, 160, 58, 206024),
        BuildingValueData(136115, 120870, 137200, 65335, 4, 192, 56, 239287),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::MainBuilding, 10),
            BuildingRequirement(BuildingName::Academy, 10),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static RESIDENCE: BuildingData = BuildingData {
    data: &[
        BuildingValueData(580, 460, 350, 180, 1, 2, 100, 2000),
        BuildingValueData(740, 590, 450, 230, 1, 3, 90, 2620),
        BuildingValueData(950, 755, 575, 295, 1, 3, 81, 3340),
        BuildingValueData(1215, 965, 735, 375, 1, 4, 73, 4170),
        BuildingValueData(1555, 1235, 940, 485, 1, 5, 66, 5140),
        BuildingValueData(1995, 1580, 1205, 620, 1, 6, 59, 6260),
        BuildingValueData(2550, 2025, 1540, 790, 1, 7, 53, 7570),
        BuildingValueData(3265, 2590, 1970, 1015, 1, 9, 48, 9080),
        BuildingValueData(4180, 3315, 2520, 1295, 1, 10, 43, 10830),
        BuildingValueData(5350, 4245, 3230, 1660, 1, 12, 39, 12860),
        BuildingValueData(6845, 5430, 4130, 2125, 2, 15, 35, 15220),
        BuildingValueData(8765, 6950, 5290, 2720, 2, 18, 32, 17950),
        BuildingValueData(11220, 8900, 6770, 3480, 2, 21, 28, 21130),
        BuildingValueData(14360, 11390, 8665, 4455, 2, 26, 25, 24810),
        BuildingValueData(18380, 14580, 11090, 5705, 2, 31, 23, 29080),
        BuildingValueData(23530, 18660, 14200, 7300, 2, 37, 21, 34030),
        BuildingValueData(30115, 23885, 18175, 9345, 2, 44, 19, 39770),
        BuildingValueData(38550, 30570, 23260, 11965, 2, 53, 17, 46440),
        BuildingValueData(49340, 39130, 29775, 15315, 2, 64, 15, 54170),
        BuildingValueData(63155, 50090, 38110, 19600, 2, 77, 13, 63130),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[BuildingRequirement(BuildingName::MainBuilding, 5)],
        conflicts: &[BuildingConflict(BuildingName::Palace)],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static PALACE: BuildingData = BuildingData {
    data: &[
        BuildingValueData(550, 800, 750, 250, 1, 6, 100, 5000),
        BuildingValueData(705, 1025, 960, 320, 1, 7, 90, 6100),
        BuildingValueData(900, 1310, 1230, 410, 1, 9, 81, 7380),
        BuildingValueData(1155, 1680, 1575, 525, 1, 10, 73, 8860),
        BuildingValueData(1475, 2145, 2015, 670, 1, 12, 66, 10570),
        BuildingValueData(1890, 2750, 2575, 860, 1, 15, 59, 12560),
        BuildingValueData(2420, 3520, 3300, 1100, 1, 18, 53, 14880),
        BuildingValueData(3095, 4505, 4220, 1405, 1, 21, 48, 17560),
        BuildingValueData(3965, 5765, 5405, 1800, 1, 26, 43, 20660),
        BuildingValueData(5075, 7380, 6920, 2305, 1, 31, 39, 24270),
        BuildingValueData(6495, 9445, 8855, 2950, 2, 37, 35, 28450),
        BuildingValueData(8310, 12090, 11335, 3780, 2, 45, 32, 33306),
        BuildingValueData(10640, 15475, 14505, 4835, 2, 53, 28, 38935),
        BuildingValueData(13615, 19805, 18570, 6190, 2, 64, 25, 45465),
        BuildingValueData(17430, 25355, 23770, 7925, 2, 77, 23, 53039),
        BuildingValueData(22310, 32450, 30425, 10140, 2, 92, 21, 61825),
        BuildingValueData(28560, 41540, 38940, 12980, 2, 111, 19, 72018),
        BuildingValueData(36555, 53170, 49845, 16615, 2, 133, 17, 83840),
        BuildingValueData(46790, 68055, 63805, 21270, 2, 160, 15, 97555),
        BuildingValueData(59890, 87110, 81675, 27225, 2, 192, 13, 113464),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::MainBuilding, 5),
            BuildingRequirement(BuildingName::Embassy, 1),
        ],
        conflicts: &[BuildingConflict(BuildingName::Residence)],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static TREASURY: BuildingData = BuildingData {
    data: &[
        BuildingValueData(2880, 2740, 2580, 990, 4, 7, 0, 8000),
        BuildingValueData(3630, 3450, 3250, 1245, 2, 9, 0, 9580),
        BuildingValueData(4570, 4350, 4095, 1570, 2, 10, 0, 11410),
        BuildingValueData(5760, 5480, 5160, 1980, 2, 12, 0, 13540),
        BuildingValueData(7260, 6905, 6505, 2495, 2, 15, 0, 16010),
        BuildingValueData(9145, 8700, 8195, 3145, 3, 18, 0, 18870),
        BuildingValueData(11525, 10965, 10325, 3960, 3, 21, 0, 22180),
        BuildingValueData(14520, 13815, 13010, 4990, 3, 26, 0, 26030),
        BuildingValueData(18295, 17405, 16390, 6290, 3, 31, 0, 30500),
        BuildingValueData(23055, 21930, 20650, 7925, 3, 37, 1, 35680),
        BuildingValueData(29045, 27635, 26020, 9985, 3, 45, 1, 41690),
        BuildingValueData(36600, 34820, 32785, 12580, 3, 53, 1, 48660),
        BuildingValueData(46115, 43875, 41310, 15850, 3, 64, 1, 56740),
        BuildingValueData(58105, 55280, 52050, 19975, 3, 77, 1, 66120),
        BuildingValueData(73210, 69655, 65585, 25165, 3, 92, 1, 77000),
        BuildingValueData(92245, 87760, 82640, 31710, 4, 111, 1, 89620),
        BuildingValueData(116230, 110580, 104125, 39955, 4, 133, 1, 104260),
        BuildingValueData(146450, 139330, 131195, 50340, 4, 160, 1, 121240),
        BuildingValueData(184530, 175560, 165305, 63430, 4, 192, 1, 140940),
        BuildingValueData(232505, 221205, 208285, 79925, 4, 230, 1, 163790),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[BuildingRequirement(BuildingName::MainBuilding, 10)],
        conflicts: &[BuildingConflict(BuildingName::WonderOfTheWorld)],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static TRADE_OFFICE: BuildingData = BuildingData {
    data: &[
        BuildingValueData(1400, 1330, 1200, 400, 3, 4, 110, 3000),
        BuildingValueData(1790, 1700, 1535, 510, 2, 4, 120, 3780),
        BuildingValueData(2295, 2180, 1965, 655, 2, 5, 130, 4680),
        BuildingValueData(2935, 2790, 2515, 840, 2, 6, 140, 5730),
        BuildingValueData(3760, 3570, 3220, 1075, 2, 7, 150, 6950),
        BuildingValueData(4810, 4570, 4125, 1375, 2, 9, 160, 8360),
        BuildingValueData(6155, 5850, 5280, 1760, 2, 11, 170, 10000),
        BuildingValueData(7880, 7485, 6755, 2250, 2, 13, 180, 11900),
        BuildingValueData(10090, 9585, 8645, 2880, 2, 15, 190, 14110),
        BuildingValueData(12915, 12265, 11070, 3690, 2, 19, 200, 16660),
        BuildingValueData(16530, 15700, 14165, 4720, 3, 22, 210, 19630),
        BuildingValueData(21155, 20100, 18135, 6045, 3, 27, 220, 23070),
        BuildingValueData(27080, 25725, 23210, 7735, 3, 32, 230, 27060),
        BuildingValueData(34660, 32930, 29710, 9905, 3, 39, 240, 31690),
        BuildingValueData(44370, 42150, 38030, 12675, 3, 46, 250, 37060),
        BuildingValueData(56790, 53950, 48680, 16225, 3, 55, 260, 43290),
        BuildingValueData(72690, 69060, 62310, 20770, 3, 67, 270, 50520),
        BuildingValueData(93045, 88395, 79755, 26585, 3, 80, 280, 58900),
        BuildingValueData(119_100, 113_145, 102_085, 34030, 3, 96, 290, 68630),
        BuildingValueData(152_445, 144_825, 130_670, 43555, 3, 115, 300, 79910),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::Marketplace, 20),
            BuildingRequirement(BuildingName::Stable, 10),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static GREAT_BARRACKS: BuildingData = BuildingData {
    data: &[
        BuildingValueData(630, 420, 780, 360, 4, 1, 100, 2000),
        BuildingValueData(805, 540, 1000, 460, 2, 1, 90, 2620),
        BuildingValueData(1035, 690, 1275, 585, 2, 2, 81, 3340),
        BuildingValueData(1320, 885, 1635, 750, 2, 2, 73, 4170),
        BuildingValueData(1695, 1125, 2100, 960, 2, 2, 66, 5140),
        BuildingValueData(2160, 1440, 2685, 1230, 3, 3, 59, 6260),
        BuildingValueData(2775, 1845, 3435, 1590, 3, 4, 53, 7570),
        BuildingValueData(3540, 2370, 4395, 2025, 3, 4, 48, 9080),
        BuildingValueData(4545, 3030, 5625, 2595, 3, 5, 43, 10830),
        BuildingValueData(5805, 3870, 7200, 3315, 3, 6, 39, 12860),
        BuildingValueData(7460, 4965, 9210, 4245, 3, 7, 35, 15220),
        BuildingValueData(9525, 6345, 11790, 5445, 3, 9, 31, 17950),
        BuildingValueData(12180, 8130, 15090, 6960, 3, 11, 28, 21130),
        BuildingValueData(15600, 10395, 19305, 8910, 3, 13, 25, 24810),
        BuildingValueData(19965, 13305, 24720, 11415, 3, 15, 23, 29080),
        BuildingValueData(25560, 17040, 31635, 14610, 4, 18, 21, 34030),
        BuildingValueData(32715, 21810, 40500, 18690, 4, 22, 19, 39770),
        BuildingValueData(41870, 27915, 51840, 23925, 4, 27, 17, 46440),
        BuildingValueData(53595, 35730, 66360, 30630, 4, 32, 15, 54170),
        BuildingValueData(68595, 45735, 84930, 39195, 4, 38, 13, 63130),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[BuildingRequirement(BuildingName::Barracks, 20)],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[BuildingConstraint::NonCapital],
        allow_multiple: false,
    },
};

static GREAT_STABLE: BuildingData = BuildingData {
    data: &[
        BuildingValueData(780, 420, 660, 300, 5, 2, 100, 2200),
        BuildingValueData(1000, 540, 845, 385, 3, 3, 90, 2850),
        BuildingValueData(1280, 690, 1080, 490, 3, 3, 81, 3610),
        BuildingValueData(1635, 880, 1385, 630, 3, 4, 73, 4490),
        BuildingValueData(2095, 1125, 1770, 805, 3, 5, 66, 5500),
        BuildingValueData(2680, 1445, 2270, 1030, 3, 6, 59, 6680),
        BuildingValueData(3430, 1845, 2905, 1320, 3, 7, 53, 8050),
        BuildingValueData(4390, 2365, 3715, 1690, 3, 9, 48, 9640),
        BuildingValueData(5620, 3025, 4755, 2160, 3, 10, 43, 11480),
        BuildingValueData(7195, 3875, 6085, 2765, 3, 12, 39, 13620),
        BuildingValueData(9210, 4960, 7790, 3540, 4, 15, 35, 16100),
        BuildingValueData(11785, 6345, 9975, 4535, 4, 18, 31, 18980),
        BuildingValueData(15085, 8125, 12765, 5805, 4, 21, 28, 22310),
        BuildingValueData(19310, 10400, 16340, 7430, 4, 26, 25, 26180),
        BuildingValueData(24720, 13310, 20915, 9505, 4, 31, 23, 30670),
        BuildingValueData(31640, 17035, 26775, 12170, 4, 37, 21, 35880),
        BuildingValueData(40500, 21810, 34270, 15575, 4, 44, 19, 41920),
        BuildingValueData(51840, 27915, 43865, 19940, 4, 53, 17, 48930),
        BuildingValueData(66355, 35730, 56145, 25520, 4, 64, 15, 57060),
        BuildingValueData(84935, 45735, 71870, 32665, 4, 77, 13, 66490),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[BuildingRequirement(BuildingName::Stable, 20)],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[BuildingConstraint::NonCapital],
        allow_multiple: false,
    },
};

static CITY_WALL: BuildingData = BuildingData {
    data: &[
        BuildingValueData(70, 90, 170, 70, 0, 1, 3, 2000),
        BuildingValueData(90, 115, 220, 90, 0, 1, 6, 2620),
        BuildingValueData(115, 145, 280, 115, 0, 2, 9, 3340),
        BuildingValueData(145, 190, 355, 145, 0, 2, 13, 4170),
        BuildingValueData(190, 240, 455, 190, 0, 2, 16, 5140),
        BuildingValueData(240, 310, 585, 240, 1, 3, 19, 6260),
        BuildingValueData(310, 395, 750, 310, 1, 4, 23, 7570),
        BuildingValueData(395, 505, 955, 395, 1, 4, 27, 9080),
        BuildingValueData(505, 650, 1225, 505, 1, 5, 30, 10830),
        BuildingValueData(645, 830, 1570, 645, 1, 6, 34, 12860),
        BuildingValueData(825, 1065, 2005, 825, 1, 7, 38, 15220),
        BuildingValueData(1060, 1360, 2570, 1060, 1, 9, 43, 17950),
        BuildingValueData(1355, 1740, 3290, 1355, 1, 11, 47, 21130),
        BuildingValueData(1735, 2230, 4210, 1735, 1, 13, 51, 24810),
        BuildingValueData(2220, 2850, 5390, 2220, 1, 15, 56, 29080),
        BuildingValueData(2840, 3650, 6895, 2840, 2, 18, 60, 34030),
        BuildingValueData(3635, 4675, 8825, 3635, 2, 22, 65, 39770),
        BuildingValueData(4650, 5980, 11300, 4650, 2, 27, 70, 46440),
        BuildingValueData(5955, 7655, 14460, 5955, 2, 32, 75, 54170),
        BuildingValueData(7620, 9800, 18510, 7620, 2, 38, 81, 63130),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[],
        conflicts: &[],
        tribes: &[Tribe::Roman],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static EARTH_WALL: BuildingData = BuildingData {
    data: &[
        BuildingValueData(120, 200, 0, 80, 0, 1, 2, 2000),
        BuildingValueData(155, 255, 0, 100, 0, 1, 4, 2620),
        BuildingValueData(195, 330, 0, 130, 0, 2, 6, 3340),
        BuildingValueData(250, 420, 0, 170, 0, 2, 8, 4170),
        BuildingValueData(320, 535, 0, 215, 0, 2, 10, 5140),
        BuildingValueData(410, 685, 0, 275, 1, 3, 13, 6260),
        BuildingValueData(530, 880, 0, 350, 1, 4, 15, 7570),
        BuildingValueData(675, 1125, 0, 450, 1, 4, 17, 9080),
        BuildingValueData(865, 1440, 0, 575, 1, 5, 20, 10830),
        BuildingValueData(1105, 1845, 0, 740, 1, 6, 22, 12860),
        BuildingValueData(1415, 2360, 0, 945, 1, 7, 24, 15220),
        BuildingValueData(1815, 3020, 0, 1210, 1, 9, 27, 17950),
        BuildingValueData(2320, 3870, 0, 1545, 1, 11, 29, 21130),
        BuildingValueData(2970, 4950, 0, 1980, 1, 13, 32, 24810),
        BuildingValueData(3805, 6340, 0, 2535, 1, 15, 35, 29080),
        BuildingValueData(4870, 8115, 0, 3245, 2, 18, 37, 34030),
        BuildingValueData(6230, 10385, 0, 4155, 2, 22, 40, 39770),
        BuildingValueData(7975, 13290, 0, 5315, 2, 27, 43, 46440),
        BuildingValueData(10210, 17015, 0, 6805, 2, 32, 46, 54170),
        BuildingValueData(13065, 21780, 0, 8710, 2, 38, 49, 63130),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[],
        conflicts: &[],
        tribes: &[Tribe::Teuton],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static PALISADE: BuildingData = BuildingData {
    data: &[
        BuildingValueData(160, 100, 80, 60, 0, 1, 2, 2000),
        BuildingValueData(205, 130, 100, 75, 0, 1, 5, 2620),
        BuildingValueData(260, 165, 130, 100, 0, 2, 8, 3340),
        BuildingValueData(335, 210, 170, 125, 0, 2, 10, 4170),
        BuildingValueData(430, 270, 215, 160, 0, 2, 13, 5140),
        BuildingValueData(550, 345, 275, 205, 1, 3, 16, 6260),
        BuildingValueData(705, 440, 350, 265, 1, 4, 19, 7570),
        BuildingValueData(900, 565, 450, 340, 1, 4, 22, 9080),
        BuildingValueData(1155, 720, 575, 430, 1, 5, 25, 10830),
        BuildingValueData(1475, 920, 740, 555, 1, 6, 28, 12860),
        BuildingValueData(1890, 1180, 945, 710, 1, 7, 31, 15220),
        BuildingValueData(2420, 1510, 1210, 905, 1, 9, 34, 17950),
        BuildingValueData(3095, 1935, 1545, 1160, 1, 11, 38, 21130),
        BuildingValueData(3960, 2475, 1980, 1485, 1, 13, 41, 24810),
        BuildingValueData(5070, 3170, 2535, 1900, 1, 15, 45, 29080),
        BuildingValueData(6490, 4055, 3245, 2435, 2, 18, 48, 34030),
        BuildingValueData(8310, 5190, 4155, 3115, 2, 22, 52, 39770),
        BuildingValueData(10635, 6645, 5315, 3990, 2, 27, 56, 46440),
        BuildingValueData(13610, 8505, 6805, 5105, 2, 32, 60, 54170),
        BuildingValueData(17420, 10890, 8710, 6535, 2, 38, 64, 63130),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[],
        conflicts: &[],
        tribes: &[Tribe::Gaul],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static STONEMANSION_LODGE: BuildingData = BuildingData {
    data: &[
        BuildingValueData(155, 130, 125, 70, 2, 1, 110, 2200),
        BuildingValueData(200, 165, 160, 90, 1, 1, 120, 3150),
        BuildingValueData(255, 215, 205, 115, 1, 2, 130, 4260),
        BuildingValueData(325, 275, 260, 145, 1, 2, 140, 5540),
        BuildingValueData(415, 350, 335, 190, 1, 2, 150, 7020),
        BuildingValueData(535, 445, 430, 240, 2, 3, 160, 8750),
        BuildingValueData(680, 570, 550, 310, 2, 4, 170, 10750),
        BuildingValueData(875, 730, 705, 395, 2, 4, 180, 13070),
        BuildingValueData(1115, 935, 900, 505, 2, 5, 190, 15760),
        BuildingValueData(1430, 1200, 1155, 645, 2, 6, 200, 18880),
        BuildingValueData(1830, 1535, 1475, 825, 2, 7, 210, 22500),
        BuildingValueData(2340, 1965, 1890, 1060, 2, 9, 220, 26700),
        BuildingValueData(3000, 2515, 2420, 1355, 2, 11, 230, 31570),
        BuildingValueData(3840, 3220, 3095, 1735, 2, 13, 240, 37220),
        BuildingValueData(4910, 4120, 3960, 2220, 2, 15, 250, 43780),
        BuildingValueData(6290, 5275, 5070, 2840, 3, 18, 260, 51380),
        BuildingValueData(8050, 6750, 6490, 3635, 3, 22, 270, 60200),
        BuildingValueData(10300, 8640, 8310, 4650, 3, 27, 280, 70430),
        BuildingValueData(13185, 11060, 10635, 5955, 3, 32, 290, 82300),
        BuildingValueData(16880, 14155, 13610, 7620, 3, 38, 300, 96070),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::MainBuilding, 5),
            BuildingRequirement(BuildingName::Palace, 3),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[BuildingConstraint::OnlyCapital],
        allow_multiple: false,
    },
};

static BREWERY: BuildingData = BuildingData {
    data: &[
        BuildingValueData(1460, 930, 1250, 1740, 6, 5, 1, 8000),
        BuildingValueData(2045, 1300, 1750, 2435, 3, 6, 2, 9880),
        BuildingValueData(2860, 1825, 2450, 3410, 3, 7, 3, 12060),
        BuildingValueData(4005, 2550, 3430, 4775, 3, 8, 4, 14590),
        BuildingValueData(5610, 3575, 4800, 6685, 3, 10, 5, 17530),
        BuildingValueData(7850, 5000, 6725, 9360, 4, 12, 6, 20930),
        BuildingValueData(10995, 7000, 9410, 13100, 4, 14, 7, 24880),
        BuildingValueData(15390, 9805, 13175, 18340, 4, 17, 8, 29460),
        BuildingValueData(21545, 13725, 18445, 25680, 4, 21, 9, 34770),
        BuildingValueData(30165, 19215, 25825, 35950, 4, 25, 10, 40930),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::Granary, 20),
            BuildingRequirement(BuildingName::RallyPoint, 10),
        ],
        conflicts: &[],
        tribes: &[Tribe::Teuton],
        max_level: 10,
        constraints: &[BuildingConstraint::OnlyCapital],
        allow_multiple: false,
    },
};

static TRAPPER: BuildingData = BuildingData {
    data: &[
        BuildingValueData(100, 100, 100, 100, 4, 1, 10, 2000),
        BuildingValueData(130, 130, 130, 130, 2, 1, 22, 2320),
        BuildingValueData(165, 165, 165, 165, 2, 2, 35, 2690),
        BuildingValueData(210, 210, 210, 210, 2, 2, 49, 3120),
        BuildingValueData(270, 270, 270, 270, 2, 2, 64, 3620),
        BuildingValueData(345, 345, 345, 345, 3, 3, 80, 4200),
        BuildingValueData(440, 440, 440, 440, 3, 4, 97, 4870),
        BuildingValueData(565, 565, 565, 565, 3, 4, 115, 5650),
        BuildingValueData(720, 720, 720, 720, 3, 5, 134, 6560),
        BuildingValueData(920, 920, 920, 920, 3, 6, 154, 7610),
        BuildingValueData(1180, 1180, 1180, 1180, 3, 7, 175, 8820),
        BuildingValueData(1510, 1510, 1510, 1510, 3, 9, 196, 10230),
        BuildingValueData(1935, 1935, 1935, 1935, 3, 11, 218, 11870),
        BuildingValueData(2475, 2475, 2475, 2475, 3, 13, 241, 13770),
        BuildingValueData(3170, 3170, 3170, 3170, 3, 15, 265, 15980),
        BuildingValueData(4055, 4055, 4055, 4055, 4, 18, 290, 18530),
        BuildingValueData(5190, 5190, 5190, 5190, 4, 22, 316, 21500),
        BuildingValueData(6645, 6645, 6645, 6645, 4, 27, 343, 24940),
        BuildingValueData(8505, 8505, 8505, 8505, 4, 32, 371, 28930),
        BuildingValueData(10890, 10890, 10890, 10890, 4, 38, 400, 33550),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[BuildingRequirement(BuildingName::RallyPoint, 1)],
        conflicts: &[],
        tribes: &[Tribe::Gaul],
        max_level: 20,
        constraints: &[BuildingConstraint::OnlyCapital],
        allow_multiple: true,
    },
};

static HERO_MANSION: BuildingData = BuildingData {
    data: &[
        BuildingValueData(700, 670, 700, 240, 2, 1, 0, 2300),
        BuildingValueData(930, 890, 930, 320, 1, 1, 0, 2670),
        BuildingValueData(1240, 1185, 1240, 425, 1, 2, 0, 3090),
        BuildingValueData(1645, 1575, 1645, 565, 1, 2, 0, 3590),
        BuildingValueData(2190, 2095, 2190, 750, 1, 2, 0, 4160),
        BuildingValueData(2915, 2790, 2915, 1000, 2, 3, 0, 4830),
        BuildingValueData(3875, 3710, 3875, 1330, 2, 4, 0, 5600),
        BuildingValueData(5155, 4930, 5155, 1765, 2, 4, 0, 6500),
        BuildingValueData(6855, 6560, 6855, 2350, 2, 5, 0, 7540),
        BuildingValueData(9115, 8725, 9115, 3125, 2, 6, 1, 8750),
        BuildingValueData(12125, 11605, 12125, 4155, 2, 7, 1, 10150),
        BuildingValueData(16125, 15435, 16125, 5530, 2, 9, 1, 11770),
        BuildingValueData(21445, 20525, 21445, 7350, 2, 11, 1, 13650),
        BuildingValueData(28520, 27300, 28520, 9780, 2, 13, 1, 15840),
        BuildingValueData(37935, 36310, 37935, 13005, 2, 15, 2, 18370),
        BuildingValueData(50450, 48290, 50450, 17300, 3, 18, 2, 21310),
        BuildingValueData(67100, 64225, 67100, 23005, 3, 22, 2, 24720),
        BuildingValueData(89245, 85420, 89245, 30600, 3, 27, 2, 28680),
        BuildingValueData(118_695, 113_605, 118_695, 40695, 3, 32, 2, 33260),
        BuildingValueData(157_865, 151_095, 157_865, 54125, 3, 38, 3, 38590),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::RallyPoint, 1),
            BuildingRequirement(BuildingName::MainBuilding, 3),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static GREAT_WAREHOUSE: BuildingData = BuildingData {
    data: &[
        BuildingValueData(650, 800, 450, 200, 1, 1, 3600, 9000),
        BuildingValueData(830, 1025, 575, 255, 1, 1, 5100, 10740),
        BuildingValueData(1065, 1310, 735, 330, 1, 2, 6900, 12760),
        BuildingValueData(1365, 1680, 945, 420, 1, 2, 9300, 15100),
        BuildingValueData(1745, 2145, 1210, 535, 1, 2, 12000, 17820),
        BuildingValueData(2235, 2750, 1545, 685, 1, 3, 15000, 20970),
        BuildingValueData(2860, 3520, 1980, 880, 1, 4, 18900, 24620),
        BuildingValueData(3660, 4505, 2535, 1125, 1, 4, 23400, 28860),
        BuildingValueData(4685, 5765, 3245, 1440, 1, 5, 28800, 33780),
        BuildingValueData(5995, 7380, 4150, 1845, 1, 6, 35400, 39480),
        BuildingValueData(7675, 9445, 5315, 2360, 2, 7, 43200, 46100),
        BuildingValueData(9825, 12090, 6800, 3020, 2, 9, 52800, 53780),
        BuildingValueData(12575, 15475, 8705, 3870, 2, 11, 64200, 62680),
        BuildingValueData(16095, 19805, 11140, 4950, 2, 13, 77700, 73010),
        BuildingValueData(20600, 25355, 14260, 6340, 2, 15, 93900, 84990),
        BuildingValueData(26365, 32450, 18255, 8115, 2, 18, 113_700, 98890),
        BuildingValueData(33750, 41540, 23365, 10385, 2, 22, 137_100, 115_010),
        BuildingValueData(43200, 53170, 29910, 13290, 2, 27, 165_300, 133_710),
        BuildingValueData(55295, 68055, 38280, 17015, 2, 32, 199_200, 155_400),
        BuildingValueData(70780, 87110, 49000, 21780, 2, 38, 240_000, 180_570),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::WonderOfTheWorld, 0),
            BuildingRequirement(BuildingName::MainBuilding, 10),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: true,
    },
};

static GREAT_GRANARY: BuildingData = BuildingData {
    data: &[
        BuildingValueData(400, 500, 350, 100, 1, 1, 3600, 7000),
        BuildingValueData(510, 640, 450, 130, 1, 1, 5100, 8420),
        BuildingValueData(655, 820, 575, 165, 1, 2, 6900, 10070),
        BuildingValueData(840, 1050, 735, 210, 1, 2, 9300, 11980),
        BuildingValueData(1075, 1340, 940, 270, 1, 2, 12000, 14190),
        BuildingValueData(1375, 1720, 1205, 345, 1, 3, 15000, 16770),
        BuildingValueData(1760, 2200, 1540, 440, 1, 4, 18900, 19750),
        BuildingValueData(2250, 2815, 1970, 565, 1, 4, 23400, 23210),
        BuildingValueData(2880, 3605, 2520, 720, 1, 5, 28800, 27220),
        BuildingValueData(3690, 4610, 3230, 920, 1, 6, 35400, 31880),
        BuildingValueData(4720, 5905, 4130, 1180, 2, 7, 43200, 37280),
        BuildingValueData(6045, 7555, 5290, 1510, 2, 9, 52800, 43540),
        BuildingValueData(7735, 9670, 6770, 1935, 2, 11, 64200, 50810),
        BuildingValueData(9905, 12380, 8665, 2475, 2, 13, 77700, 59240),
        BuildingValueData(12675, 15845, 11090, 3170, 2, 15, 93900, 69010),
        BuildingValueData(16225, 20280, 14200, 4055, 2, 18, 113_700, 80360),
        BuildingValueData(20770, 25960, 18175, 5190, 2, 22, 137_100, 93510),
        BuildingValueData(26585, 33230, 23260, 6645, 2, 27, 165_300, 108_780),
        BuildingValueData(34030, 42535, 29775, 8505, 2, 32, 199_200, 126_480),
        BuildingValueData(43555, 54445, 38110, 10890, 2, 38, 240_000, 147_020),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::WonderOfTheWorld, 0),
            BuildingRequirement(BuildingName::MainBuilding, 10),
        ],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[],
        allow_multiple: true,
    },
};

static HORSE_DRINKING_TROUGH: BuildingData = BuildingData {
    data: &[
        BuildingValueData(400, 500, 350, 100, 1, 1, 3600, 7000),
        BuildingValueData(510, 640, 450, 130, 1, 1, 5100, 8420),
        BuildingValueData(655, 820, 575, 165, 1, 2, 6900, 10070),
        BuildingValueData(840, 1050, 735, 210, 1, 2, 9300, 11980),
        BuildingValueData(1075, 1340, 940, 270, 1, 2, 12000, 14190),
        BuildingValueData(1375, 1720, 1205, 345, 1, 3, 15000, 16770),
        BuildingValueData(1760, 2200, 1540, 440, 1, 4, 18900, 19750),
        BuildingValueData(2250, 2815, 1970, 565, 1, 4, 23400, 23210),
        BuildingValueData(2880, 3605, 2520, 720, 1, 5, 28800, 27220),
        BuildingValueData(3690, 4610, 3230, 920, 1, 6, 35400, 31880),
        BuildingValueData(4720, 5905, 4130, 1180, 2, 7, 43200, 37280),
        BuildingValueData(6045, 7555, 5290, 1510, 2, 9, 52800, 43540),
        BuildingValueData(7735, 9670, 6770, 1935, 2, 11, 64200, 50810),
        BuildingValueData(9905, 12380, 8665, 2475, 2, 13, 77700, 59240),
        BuildingValueData(12675, 15845, 11090, 3170, 2, 15, 93900, 69010),
        BuildingValueData(16225, 20280, 14200, 4055, 2, 18, 113_700, 80360),
        BuildingValueData(20770, 25960, 18175, 5190, 2, 22, 137_100, 93510),
        BuildingValueData(26585, 33230, 23260, 6645, 2, 27, 165_300, 108_780),
        BuildingValueData(34030, 42535, 29775, 8505, 2, 32, 199_200, 126_480),
        BuildingValueData(43555, 54445, 38110, 10890, 2, 38, 240_000, 147_020),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[
            BuildingRequirement(BuildingName::Stable, 20),
            BuildingRequirement(BuildingName::RallyPoint, 10),
        ],
        conflicts: &[],
        tribes: &[Tribe::Roman],
        max_level: 20,
        constraints: &[],
        allow_multiple: false,
    },
};

static GREAT_WORKSHOP: BuildingData = BuildingData {
    data: &[
        BuildingValueData(1380, 1530, 1800, 960, 3, 4, 100, 3000),
        BuildingValueData(1770, 1915, 2310, 1230, 2, 4, 90, 3780),
        BuildingValueData(2215, 2505, 2955, 1575, 2, 5, 81, 4680),
        BuildingValueData(2895, 3210, 3780, 2010, 2, 6, 73, 5730),
        BuildingValueData(3705, 4110, 4830, 2580, 2, 7, 66, 6950),
        BuildingValueData(4740, 5250, 6180, 3300, 2, 9, 59, 8360),
        BuildingValueData(6075, 6735, 7920, 4215, 2, 11, 53, 10000),
        BuildingValueData(7730, 8610, 10140, 5600, 2, 13, 48, 11900),
        BuildingValueData(9945, 11025, 12975, 4615, 2, 15, 43, 14110),
        BuildingValueData(12735, 14115, 16605, 8850, 2, 19, 39, 16660),
        BuildingValueData(16290, 18060, 23415, 11340, 3, 22, 35, 19630),
        BuildingValueData(20850, 23115, 27195, 14505, 3, 27, 31, 23070),
        BuildingValueData(26700, 29595, 34815, 18570, 3, 32, 28, 27060),
        BuildingValueData(34170, 37875, 44565, 23775, 3, 39, 25, 31690),
        BuildingValueData(43740, 48495, 57045, 30420, 3, 46, 23, 37060),
        BuildingValueData(55980, 62070, 73020, 38940, 3, 55, 21, 43290),
        BuildingValueData(71655, 79440, 93465, 49485, 3, 67, 19, 50520),
        BuildingValueData(91710, 101_685, 119_625, 63810, 3, 80, 17, 58900),
        BuildingValueData(105_650, 117_140, 137_810, 73510, 3, 96, 15, 68630),
        BuildingValueData(125_225, 138_840, 159_995, 87090, 3, 115, 13, 79910),
    ],
    group: BuildingGroup::Military,
    rules: BuildingRules {
        requirements: &[BuildingRequirement(BuildingName::Workshop, 20)],
        conflicts: &[],
        tribes: &[],
        max_level: 20,
        constraints: &[BuildingConstraint::NonCapital],
        allow_multiple: false,
    },
};

static WONDER_OF_THW_WORLD: BuildingData = BuildingData {
    data: &[
        BuildingValueData(66700, 69050, 72200, 13200, 1, 0, 0, 18000),
        BuildingValueData(68535, 70950, 74185, 13565, 1, 0, 0, 18850),
        BuildingValueData(70420, 72900, 76225, 13935, 1, 0, 0, 19720),
        BuildingValueData(72355, 74905, 78320, 14320, 1, 0, 0, 20590),
        BuildingValueData(74345, 76965, 80475, 14715, 1, 0, 0, 21480),
        BuildingValueData(76390, 79080, 82690, 15120, 1, 0, 0, 22380),
        BuildingValueData(78490, 81255, 84965, 15535, 1, 0, 0, 23290),
        BuildingValueData(80650, 83490, 87300, 15960, 1, 0, 0, 24220),
        BuildingValueData(82865, 85785, 89700, 16400, 1, 0, 0, 25160),
        BuildingValueData(85145, 88145, 92165, 16850, 1, 0, 0, 26110),
        BuildingValueData(87485, 90570, 94700, 17315, 2, 0, 0, 27080),
        BuildingValueData(89895, 93060, 97305, 17790, 2, 0, 0, 28060),
        BuildingValueData(92365, 95620, 99980, 18280, 2, 0, 0, 29050),
        BuildingValueData(94905, 98250, 102_730, 18780, 2, 0, 0, 30060),
        BuildingValueData(97515, 100_950, 105_555, 19300, 2, 0, 0, 31080),
        BuildingValueData(100_195, 103_725, 108_460, 19830, 2, 0, 0, 32110),
        BuildingValueData(102_950, 106_580, 111_440, 20375, 2, 0, 0, 33160),
        BuildingValueData(105_785, 109_510, 114_505, 20935, 2, 0, 0, 34230),
        BuildingValueData(108_690, 112_520, 117_655, 21510, 2, 0, 0, 35300),
        BuildingValueData(111_680, 115_615, 120_890, 22100, 2, 0, 0, 36400),
        BuildingValueData(114_755, 118_795, 124_215, 22710, 3, 0, 0, 37510),
        BuildingValueData(117_910, 122_060, 127_630, 23335, 3, 0, 0, 38630),
        BuildingValueData(121_150, 125_420, 131_140, 23975, 3, 0, 0, 39770),
        BuildingValueData(124_480, 128_870, 134_745, 24635, 3, 0, 0, 40930),
        BuildingValueData(127_905, 132_410, 138_455, 25315, 3, 0, 0, 42100),
        BuildingValueData(131_425, 136_055, 142_260, 26010, 3, 0, 0, 43290),
        BuildingValueData(135_035, 139_795, 146_170, 26725, 3, 0, 0, 44500),
        BuildingValueData(138_750, 143_640, 150_190, 27460, 3, 0, 0, 45720),
        BuildingValueData(142_565, 147_590, 154_320, 28215, 3, 0, 0, 46960),
        BuildingValueData(146_485, 151_650, 158_565, 28990, 3, 0, 0, 48220),
        BuildingValueData(150_515, 155_820, 162_925, 29785, 4, 0, 0, 49500),
        BuildingValueData(154_655, 160_105, 167_405, 30605, 4, 0, 0, 50790),
        BuildingValueData(158_910, 164_505, 172_010, 31450, 4, 0, 0, 52100),
        BuildingValueData(163_275, 169_030, 176_740, 32315, 4, 0, 0, 53430),
        BuildingValueData(167_770, 173_680, 181_600, 33200, 4, 0, 0, 54780),
        BuildingValueData(172_380, 178_455, 186_595, 34115, 4, 0, 0, 56140),
        BuildingValueData(177_120, 183_360, 191_725, 35055, 4, 0, 0, 57530),
        BuildingValueData(181_995, 188_405, 197_000, 36015, 4, 0, 0, 58940),
        BuildingValueData(186_995, 193_585, 202_415, 37005, 4, 0, 0, 60360),
        BuildingValueData(192_140, 198_910, 207_985, 38025, 4, 0, 0, 61810),
        BuildingValueData(197_425, 204_380, 213_705, 39070, 5, 0, 0, 63270),
        BuildingValueData(202_855, 210_000, 219_580, 40145, 5, 0, 0, 64760),
        BuildingValueData(208_430, 215_775, 225_620, 41250, 5, 0, 0, 66260),
        BuildingValueData(214_165, 221_710, 231_825, 42385, 5, 0, 0, 67790),
        BuildingValueData(220_055, 227_805, 238_200, 43550, 5, 0, 0, 69340),
        BuildingValueData(226_105, 234_070, 244_750, 44745, 5, 0, 0, 70910),
        BuildingValueData(232_320, 240_505, 251_480, 45975, 5, 0, 0, 72500),
        BuildingValueData(238_710, 247_120, 258_395, 47240, 5, 0, 0, 74120),
        BuildingValueData(245_275, 253_915, 265_500, 48540, 5, 0, 0, 75760),
        BuildingValueData(252_020, 260_900, 272_800, 49875, 5, 0, 0, 77420),
        BuildingValueData(258_950, 268_075, 280_305, 51245, 6, 0, 0, 79100),
        BuildingValueData(266_070, 275_445, 288_010, 52655, 6, 0, 0, 80810),
        BuildingValueData(273_390, 283_020, 295_930, 54105, 6, 0, 0, 82540),
        BuildingValueData(280_905, 290_805, 304_070, 55590, 6, 0, 0, 84290),
        BuildingValueData(288_630, 298_800, 312_430, 57120, 6, 0, 0, 86070),
        BuildingValueData(296_570, 307_020, 321_025, 58690, 6, 0, 0, 87880),
        BuildingValueData(304_725, 315_460, 329_850, 60305, 6, 0, 0, 89710),
        BuildingValueData(313_105, 324_135, 338_925, 61965, 6, 0, 0, 91570),
        BuildingValueData(321_715, 333_050, 348_245, 63670, 6, 0, 0, 93450),
        BuildingValueData(330_565, 342_210, 357_820, 65420, 6, 0, 0, 95360),
        BuildingValueData(339_655, 351_620, 367_660, 67220, 7, 0, 0, 97290),
        BuildingValueData(348_995, 361_290, 377_770, 69065, 7, 0, 0, 99250),
        BuildingValueData(358_590, 371_225, 388_160, 70965, 7, 0, 0, 101_240),
        BuildingValueData(368_450, 381_435, 398_835, 72915, 7, 0, 0, 103_260),
        BuildingValueData(378_585, 391_925, 409_800, 74920, 7, 0, 0, 105_310),
        BuildingValueData(388_995, 402_700, 421_070, 76985, 7, 0, 0, 107_380),
        BuildingValueData(399_695, 413_775, 432_650, 79100, 7, 0, 0, 109_480),
        BuildingValueData(410_685, 425_155, 444_550, 81275, 7, 0, 0, 111_620),
        BuildingValueData(421_980, 436_845, 456_775, 83510, 7, 0, 0, 113_780),
        BuildingValueData(433_585, 448_860, 469_335, 85805, 7, 0, 0, 115_970),
        BuildingValueData(445_505, 461_205, 482_240, 88165, 8, 0, 0, 118_200),
        BuildingValueData(457_760, 473_885, 495_505, 90590, 8, 0, 0, 120_450),
        BuildingValueData(470_345, 486_920, 509_130, 93080, 8, 0, 0, 122_740),
        BuildingValueData(483_280, 500_310, 523_130, 95640, 8, 0, 0, 125_060),
        BuildingValueData(496_570, 514_065, 537_520, 98270, 8, 0, 0, 127_410),
        BuildingValueData(510_225, 528_205, 552_300, 100_975, 8, 0, 0, 129_790),
        BuildingValueData(524_260, 542_730, 567_490, 103_750, 8, 0, 0, 132_210),
        BuildingValueData(538_675, 557_655, 583_095, 106_605, 8, 0, 0, 134_660),
        BuildingValueData(553_490, 572_990, 599_130, 109_535, 8, 0, 0, 137_140),
        BuildingValueData(568_710, 588_745, 615_605, 112_550, 8, 0, 0, 139_660),
        BuildingValueData(584_350, 604_935, 632_535, 115_645, 9, 0, 0, 142_220),
        BuildingValueData(600_420, 621_575, 649_930, 118_825, 9, 0, 0, 144_810),
        BuildingValueData(616_930, 638_665, 667_800, 122_090, 9, 0, 0, 147_440),
        BuildingValueData(633_895, 656_230, 686_165, 125_450, 9, 0, 0, 150_100),
        BuildingValueData(651_330, 674_275, 705_035, 128_900, 9, 0, 0, 152_800),
        BuildingValueData(669_240, 692_820, 724_425, 132_445, 9, 0, 0, 155_540),
        BuildingValueData(687_645, 711_870, 744_345, 136_085, 9, 0, 0, 158_320),
        BuildingValueData(706_555, 731_445, 764_815, 139_830, 9, 0, 0, 161_140),
        BuildingValueData(725_985, 751_560, 785_850, 143_675, 9, 0, 0, 163_990),
        BuildingValueData(745_950, 772_230, 807_460, 147_625, 9, 0, 0, 166_890),
        BuildingValueData(766_460, 793_465, 829_665, 151_685, 10, 0, 0, 169_820),
        BuildingValueData(787_540, 815_285, 852_480, 155_855, 10, 0, 0, 172_800),
        BuildingValueData(809_195, 837_705, 875_920, 160_140, 10, 0, 0, 175_820),
        BuildingValueData(831_450, 860_745, 900_010, 164_545, 10, 0, 0, 178_880),
        BuildingValueData(854_315, 884_415, 924_760, 169_070, 10, 0, 0, 181_990),
        BuildingValueData(877_810, 908_735, 950_190, 173_720, 10, 0, 0, 185_130),
        BuildingValueData(901_950, 933_725, 976_320, 178_495, 10, 0, 0, 188_330),
        BuildingValueData(926_750, 959_405, 1_000_000, 183_405, 10, 0, 0, 191_560),
        BuildingValueData(952_235, 985_785, 1_000_000, 188_450, 10, 0, 0, 194_840),
        BuildingValueData(1_000_000, 1_000_000, 1_000_000, 193_630, 10, 0, 0, 198_170),
    ],
    group: BuildingGroup::Infrastructure,
    rules: BuildingRules {
        requirements: &[BuildingRequirement(
            BuildingName::AncientConstructionPlan,
            0,
        )],
        conflicts: &[],
        tribes: &[],
        max_level: 100,
        constraints: &[],
        allow_multiple: false,
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_build_time_with_main_building() {
        let server_speed: i8 = 1;
        let barracks = Building::new(BuildingName::Barracks, server_speed);

        // Test L0 e L1 (Fattore 1.0 -> 1000 / 1000.0)
        // assert_eq!(barracks.calculate_build_time_secs(&server_speed, &0), 2000);
        assert_eq!(barracks.calculate_build_time_secs(&server_speed, &1), 2000);

        // Test 3: Con Main Building L5 (fattore 0.864)
        let mb_level_5 = 5;
        let time_l5 = barracks.calculate_build_time_secs(&server_speed, &mb_level_5);
        let expected_l5 = (2000.0 as f64 * 0.864 as f64).floor() as u32; // 1728
        assert_eq!(time_l5, expected_l5, "MB L5 (86.4%) deve ridurre il tempo");

        // Test 4: Con Main Building L20 (fattore 0.498)
        let mb_level_20 = 20;
        let time_l20 = barracks.calculate_build_time_secs(&server_speed, &mb_level_20);
        let expected_l20 = (2000.0 as f64 * 0.498 as f64).floor() as u32; // 996
        assert_eq!(
            time_l20, expected_l20,
            "MB L20 (49.8%) deve ridurre il tempo"
        );
    }

    #[test]
    fn test_new_building() {
        let server_speed: i8 = 1;
        let wood = Building::new(BuildingName::Woodcutter, server_speed)
            .at_level(1, server_speed)
            .unwrap();

        assert_eq!(wood.level, 1);
        assert_eq!(wood.value, 5);
        assert_eq!(wood.cost().upkeep, 2);

        // Infrastructure start at level 1
        let mb = Building::new(BuildingName::MainBuilding, server_speed);
        assert_eq!(mb.level, 1);
        assert_eq!(mb.value, 1000); // Build time reduction
        assert_eq!(mb.cost().upkeep, 2);
    }

    #[test]
    fn test_at_level_with_resource_field() {
        let server_speed: i8 = 1;
        let wood = Building::new(BuildingName::Woodcutter, server_speed);

        // Get level 5
        let wood_5 = wood.at_level(5, server_speed).unwrap();
        assert_eq!(wood_5.level, 5);
        assert_eq!(wood_5.value, 33); // Production value for level 5
        assert_eq!(wood_5.cost().upkeep, 1);

        // Get level 0
        let wood_0 = wood.at_level(0, server_speed).unwrap();
        assert_eq!(wood_0.level, 0);
        assert_eq!(wood_0.value, 2); // Production value for level 0
        assert_eq!(wood_0.cost().upkeep, 0);

        // Get level beyond max (should clamp to max level)
        let wood_max = wood.at_level(99, server_speed).unwrap();
        let max_level = get_building_data(&BuildingName::Woodcutter)
            .unwrap()
            .rules
            .max_level;
        assert_eq!(wood_max.level, max_level);
        assert_eq!(wood_max.level, 20);
    }

    #[test]
    fn test_at_level_with_building() {
        let server_speed: i8 = 1;
        let bakery = Building::new(BuildingName::Bakery, server_speed);

        // Get level 5
        let bakery_5 = bakery.at_level(5, server_speed).unwrap();
        assert_eq!(bakery_5.level, 5);
        assert_eq!(bakery_5.value, 25);
        assert_eq!(bakery_5.cost().upkeep, 3);

        // Level 0 for a non-resource building means level 1
        let bakery_0 = bakery.at_level(0, server_speed).unwrap();
        println!("==== bakery {:#?}", bakery_0);
        assert_eq!(bakery_0.level, 1);
        assert_eq!(bakery_0.value, 5);

        // Get level beyond max (should clamp to max level)
        let bakery_max = bakery.at_level(6, server_speed).unwrap();
        assert_eq!(bakery_max.level, 5);
    }
}
