use std::collections::HashMap;

use anyhow::{Error, Result};

use super::{
    army::Army,
    buildings::{Building, BuildingGroup, BuildingName},
    common::{Player, SmithyUpgrades, Tribe},
    map::{Oasis, Valley, WORLD_MAX_SIZE},
};

#[derive(Debug, Clone)]
pub struct Village {
    pub id: u64,
    pub name: String,
    pub player_id: String,
    pub valley_id: u64,
    pub tribe: Tribe,
    pub buildings: HashMap<u8, Building>,
    pub oases: Vec<Oasis>,
    pub population: u32,
    pub army: Army,
    pub reinforcements: Vec<Army>,
    pub loyalty: u8,
    pub production: VillageProduction,
    pub is_capital: bool,
    pub smithy: SmithyUpgrades,
}

impl Village {
    pub fn new(name: String, valley: &Valley, player: &Player, is_capital: bool) -> Self {
        let position = valley.position.clone();
        let village_id = position.to_id(WORLD_MAX_SIZE);
        let army = Army::new(
            village_id,
            player.id.clone(),
            player.tribe.clone(),
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        let production: VillageProduction = Default::default();
        let smithy = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let mut village = Self {
            id: village_id,
            name,
            player_id: player.id.clone(),
            valley_id: valley.id,
            tribe: player.tribe.clone(),
            buildings: HashMap::new(),
            oases: vec![],
            population: 2,
            army,
            reinforcements: vec![],
            loyalty: 100,
            production,
            is_capital,
            smithy: smithy,
        };

        village.init_village_buildings(valley);
        village.update_production();
        village
    }

    pub fn add_building(&mut self, name: BuildingName, slot_id: u8) -> Result<()> {
        // can't build on existing buildings
        for (b_slot_id, _) in self.buildings.clone() {
            if b_slot_id == slot_id {
                return Err(Error::msg("can't build on existing slot"));
            }
        }

        // village slots limit is 40: 18 resources + 21 infrastructures + 1 wall
        if self.buildings.len() == 40 {
            return Err(Error::msg("all village slots have been used"));
        }

        let building = Building::new(name);

        match building.validate_build(&self.tribe, &self.buildings, self.is_capital) {
            Err(msg) => return Err(Error::msg(msg)),
            Ok(_) => {
                self.buildings.insert(slot_id, building);
                self.update_production();
            }
        }
        Ok(())
    }

    pub fn upgrade_building(&mut self, slot_id: u8) -> Result<()> {
        match self.get_building_by_slot_id(slot_id) {
            Some(b) => match b.validate_upgrade() {
                Ok(_) => {
                    let next = b.next_level().unwrap();
                    self.buildings.insert(slot_id, next);
                    self.update_production();
                }
                Err(msg) => return Err(Error::msg(msg)),
            },
            None => return Err(Error::msg("No buildings found on this slot")),
        }
        Ok(())
    }

    pub fn downgrade_building_to_level(&mut self, slot_id: u8, level: u8) -> Result<()> {
        match self.get_building_by_slot_id(slot_id) {
            Some(b) => {
                let building = b.at_level(level)?;
                self.buildings.insert(slot_id, building);
                self.update_production();
            }
            None => return Err(Error::msg("No buildings found on this slot")),
        };
        Ok(())
    }

    pub fn destroy_building(&mut self, slot_id: u8) -> Result<()> {
        match self.get_building_by_slot_id(slot_id) {
            Some(b) => {
                if b.group == BuildingGroup::Resources {
                    b.at_level(0)?;
                    self.update_production();
                } else {
                    self.buildings.remove(&slot_id);
                }
            }
            None => return Err(Error::msg("No buildings found on this slot")),
        };
        Ok(())
    }

    pub fn get_building_by_slot_id(&self, slot_id: u8) -> Option<Building> {
        self.buildings.get(&slot_id).cloned()
    }

    // Returns a building in the village. Returns None if not present. In case of multiple buildings of same type, it returns the highest level one.
    pub fn get_building_by_name(&self, name: BuildingName) -> Option<Building> {
        self.buildings
            .clone()
            .values()
            .into_iter()
            .filter(|&x| x.name == name)
            .cloned()
            .max_by(|x, y| x.level.cmp(&y.level))
    }

    pub fn get_palace_or_residence(&self) -> Option<(Building, BuildingName)> {
        if let Some(palace) = self.get_building_by_name(BuildingName::Palace) {
            return Some((palace, BuildingName::Palace));
        }
        if let Some(residence) = self.get_building_by_name(BuildingName::Residence) {
            return Some((residence, BuildingName::Residence));
        }
        None
    }

    // Returns the current wall, if any, according to the tribe.
    pub fn get_wall(&self) -> Option<Building> {
        match self.tribe {
            Tribe::Roman => self.get_building_by_name(BuildingName::CityWall),
            Tribe::Teuton => self.get_building_by_name(BuildingName::EarthWall),
            Tribe::Gaul => self.get_building_by_name(BuildingName::Palisade),
            _ => None,
        }
    }

    pub fn get_buildings_durability(&self) -> u16 {
        match self.get_building_by_name(BuildingName::StonemansionLodge) {
            Some(b) => b.value as u16,
            None => 1,
        }
    }

    // Updates the village raw production and bonuses from buildings and oases bonuses.
    pub fn update_production(&mut self) {
        // production bonuses from infrastructures
        for (_, b) in self.buildings.clone() {
            match b.name {
                BuildingName::Woodcutter => self.production.lumber += b.value,
                BuildingName::ClayPit => self.production.clay += b.value,
                BuildingName::IronMine => self.production.iron += b.value,
                BuildingName::Cropland => self.production.crop += b.value,
                BuildingName::Sawmill => self.production.bonus.lumber += b.value as u8,
                BuildingName::Brickyard => self.production.bonus.clay += b.value as u8,
                BuildingName::IronFoundry => self.production.bonus.iron += b.value as u8,
                BuildingName::GrainMill => self.production.bonus.crop += b.value as u8,
                BuildingName::Bakery => self.production.bonus.crop += b.value as u8,
                _ => continue,
            }
        }

        // oases production bonuses
        for o in self.oases.clone() {
            let oasis_bonus = o.bonus();
            self.production.bonus.add(&oasis_bonus)
        }

        // armies upkeep
        self.production.upkeep += self.army.upkeep();
        for r in self.reinforcements.clone() {
            self.production.upkeep += r.upkeep();
        }

        self.production.update_effective_production();
    }

    fn init_village_buildings(&mut self, valley: &Valley) {
        let topology = valley.topology.clone();

        for _ in 0..topology.lumber() {
            let slot_id = self.buildings.len();
            let building = Building::new(BuildingName::Woodcutter);
            self.buildings.insert(slot_id as u8, building);
        }

        for _ in 0..topology.clay() {
            let slot_id = self.buildings.len();
            let building = Building::new(BuildingName::Woodcutter);
            self.buildings.insert(slot_id as u8, building);
        }

        for _ in 0..topology.iron() {
            let slot_id = self.buildings.len();
            let building = Building::new(BuildingName::Woodcutter);
            self.buildings.insert(slot_id as u8, building);
        }

        for _ in 0..topology.crop() {
            let slot_id = self.buildings.len();
            let building = Building::new(BuildingName::Woodcutter);
            self.buildings.insert(slot_id as u8, building);
        }
    }
}

// Gross production of a village with upkeep and bonuses values ready to apply.
#[derive(Debug, Clone, Default)]
pub struct VillageProduction {
    pub lumber: u64,
    pub clay: u64,
    pub iron: u64,
    pub crop: u64,
    pub upkeep: u64,
    pub bonus: ProductionBonus,
    pub effective: VillageEffectiveProduction,
}

impl VillageProduction {
    pub fn update_effective_production(&mut self) {
        let mut production: VillageEffectiveProduction = Default::default();

        production.lumber =
            ((self.lumber as f64) * ((self.bonus.lumber as f64 / 100.0) + 1.0)).floor() as u64;
        production.clay =
            (self.clay as f64 * ((self.bonus.clay as f64 / 100.0) + 1.0)).floor() as u64;
        production.iron =
            (self.iron as f64 * ((self.bonus.iron as f64 / 100.0) + 1.0)).floor() as u64;
        production.crop =
            (self.crop as f64 * ((self.bonus.crop as f64 / 100.0) + 1.0)).floor() as i64;
        production.crop -= self.upkeep as i64;

        self.effective = production;
    }
}

#[derive(Debug, Clone, Default)]
pub struct VillageEffectiveProduction {
    pub lumber: u64,
    pub clay: u64,
    pub iron: u64,
    pub crop: i64,
}

// Bonus to be applied to resources production.
#[derive(Debug, Clone, Default)]
pub struct ProductionBonus {
    pub lumber: u8,
    pub clay: u8,
    pub iron: u8,
    pub crop: u8,
}

impl ProductionBonus {
    pub fn add(&mut self, bonus: &ProductionBonus) {
        self.lumber = bonus.lumber;
        self.clay = bonus.clay;
        self.iron = bonus.iron;
        self.crop = bonus.crop;
    }
}
