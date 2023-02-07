use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::{
    army::Army,
    buildings::{Building, BuildingGroup, BuildingName},
    map::{Oasis, Valley, WORLD_MAX_SIZE},
    {Player, SmithyUpgrades, Tribe},
};

// TODO: add standalone rally point?
// TODO: add standalone wall?
// TODO: add reinforcements to other villages?
// TODO: add warehouse and granary total size!
#[derive(Debug, Clone)]
pub struct Village {
    pub id: u32,
    pub name: String,
    pub player_id: Uuid,
    pub valley_id: u32,
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
    pub stocks: StockCapacity,
    pub updated_at: DateTime<Utc>,
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
            smithy,
            stocks: StockCapacity {
                warehouse: 800,
                granary: 800,
            }, // FIXME: set from config according to server speed
            updated_at: Utc::now(),
        };

        village.init_village_buildings(valley);
        village.update_stats();
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

        building.validate_build(&self.tribe, &self.buildings, self.is_capital)?;
        self.buildings.insert(slot_id, building);
        self.update_stats();

        Ok(())
    }

    pub fn upgrade_building(&mut self, slot_id: u8) -> Result<()> {
        match self.get_building_by_slot_id(slot_id) {
            Some(b) => match b.validate_upgrade() {
                Ok(_) => {
                    let next = b.next_level().unwrap();
                    self.buildings.insert(slot_id, next);
                    self.update_stats();
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
                self.update_stats();
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
                    self.update_stats();
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

    // Updates the village stats (population, production, bonuses from buildings and oases, etc).
    fn update_stats(&mut self) {
        self.population = 0;
        self.production = Default::default();
        self.stocks = Default::default();

        // data from infrastructures
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
                BuildingName::Warehouse => self.stocks.warehouse += b.value,
                BuildingName::Granary => self.stocks.granary += b.value,
                _ => continue,
            }
            self.population += b.cost().upkeep;
            self.production.upkeep += b.cost().upkeep;
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

        // update effective production apllying bonuses and upkeep
        self.production.calculate_effective_production();
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
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VillageProduction {
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: u32,
    pub upkeep: u32,
    pub bonus: ProductionBonus,
    pub effective: VillageEffectiveProduction,
}

impl VillageProduction {
    pub fn calculate_effective_production(&mut self) {
        let mut ep: VillageEffectiveProduction = Default::default();

        ep.lumber =
            ((self.lumber as f64) * ((self.bonus.lumber as f64 / 100.0) + 1.0)).floor() as u32;
        ep.clay = (self.clay as f64 * ((self.bonus.clay as f64 / 100.0) + 1.0)).floor() as u32;
        ep.iron = (self.iron as f64 * ((self.bonus.iron as f64 / 100.0) + 1.0)).floor() as u32;
        ep.crop = (self.crop as f64 * ((self.bonus.crop as f64 / 100.0) + 1.0)).floor() as i64;
        ep.crop -= self.upkeep as i64;

        self.effective = ep;
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VillageEffectiveProduction {
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: i64,
}

// Bonus to be applied to resources production.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct StockCapacity {
    warehouse: u32,
    granary: u32,
}
