use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::game::models::ResourceGroup;

use super::{
    army::Army,
    buildings::{Building, BuildingGroup, BuildingName},
    map::{Oasis, Position, Valley, WORLD_MAX_SIZE},
    Player, SmithyUpgrades, Tribe,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VillageBuilding {
    pub slot_id: u8,
    pub building: Building,
}

// TODO: add standalone rally point? Not yet
// TODO: add standalone wall? Not yet
// TODO: track reinforcements to other villages? -> better to have a table for armies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Village {
    pub id: u32,
    pub name: String,
    pub player_id: Uuid,
    pub position: Position,
    pub tribe: Tribe,
    pub buildings: Vec<VillageBuilding>,
    pub oases: Vec<Oasis>,
    pub population: u32,
    pub army: Option<Army>,
    pub reinforcements: Vec<Army>,
    pub deployed_armies: Vec<Army>,
    pub loyalty: u8,
    pub production: VillageProduction,
    pub is_capital: bool,
    pub smithy: SmithyUpgrades,
    pub stocks: VillageStocks,
    pub updated_at: DateTime<Utc>,
}

impl Village {
    pub fn new(name: String, valley: &Valley, player: &Player, is_capital: bool) -> Self {
        let position = valley.position.clone();
        let village_id = position.to_id(WORLD_MAX_SIZE);

        let production: VillageProduction = Default::default();
        let smithy = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let mut village = Self {
            id: village_id,
            name,
            position,
            player_id: player.id.clone(),
            tribe: player.tribe.clone(),
            buildings: vec![],
            oases: vec![],
            population: 2,
            army: None,
            reinforcements: vec![],
            deployed_armies: vec![],
            loyalty: 100,
            production,
            is_capital,
            smithy,
            stocks: Default::default(),
            updated_at: Utc::now(),
        };

        // FIXME: either fix the method return value or this method one.
        village.init_village_buildings(valley).unwrap();
        village.update_state();
        village
    }

    pub fn add_building(&mut self, name: BuildingName, slot_id: u8) -> Result<()> {
        // can't build on existing buildings
        if let Some(_) = self.get_building_by_slot_id(slot_id) {
            return Err(Error::msg("can't build on existing slot"));
        }

        // village slots limit is 40: 18 resources + 21 infrastructures + 1 wall
        if self.buildings.len() == 40 {
            return Err(Error::msg("all village slots have been used"));
        }

        let building = Building::new(name);

        building.validate_build(&self.tribe, &self.buildings, self.is_capital)?;

        self.buildings.append(&mut vec![VillageBuilding {
            slot_id,
            building: building.clone(),
        }]);
        self.update_state();

        Ok(())
    }

    pub fn upgrade_building(&mut self, slot_id: u8) -> Result<()> {
        match self.get_building_by_slot_id(slot_id) {
            Some(b) => match b.validate_upgrade() {
                Ok(_) => {
                    let next = b.next_level().unwrap();
                    self.buildings.append(&mut vec![VillageBuilding {
                        slot_id,
                        building: next,
                    }]);
                    self.update_state();
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
                self.buildings
                    .append(&mut vec![VillageBuilding { slot_id, building }]);
                self.update_state();
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
                    self.update_state();
                } else {
                    let _ = self
                        .buildings
                        .iter()
                        .filter_map(|vb| {
                            if vb.slot_id != slot_id {
                                Some(vb)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                }
            }
            None => return Err(Error::msg("No buildings found on this slot")),
        };
        Ok(())
    }

    pub fn get_building_by_slot_id(&self, slot_id: u8) -> Option<Building> {
        self.buildings
            .iter()
            .find(|&x| x.slot_id == slot_id)
            .map(|x| x.building.clone())
    }

    // Returns a building in the village. Returns None if not present. In case of multiple buildings of same type, it returns the highest level one.
    pub fn get_building_by_name(&self, name: BuildingName) -> Option<Building> {
        if let Some(village_building) = self
            .buildings
            .iter()
            .filter(|&x| x.building.name == name)
            .cloned()
            .max_by(|x, y| x.building.level.cmp(&y.building.level))
        {
            return Some(village_building.building);
        }
        None
    }

    pub fn get_random_buildings(&self, count: usize) -> Vec<Building> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let mut rng = thread_rng();
        let buildings: Vec<Building> = self
            .buildings
            .iter()
            .map(|vb| vb.building.clone())
            .collect();

        let sampled_buildings = buildings
            .choose_multiple(&mut rng, count)
            .cloned()
            .collect::<Vec<Building>>();

        sampled_buildings
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

    pub fn get_wall_name(&self) -> Option<BuildingName> {
        match self.tribe {
            Tribe::Roman => Some(BuildingName::CityWall),
            Tribe::Teuton => Some(BuildingName::EarthWall),
            Tribe::Gaul => Some(BuildingName::Palisade),
            _ => None,
        }
    }

    pub fn get_wall_defense_bonus(&self) -> f64 {
        if let Some(wall) = self.get_wall() {
            let tribe_factor: f64 = match self.tribe {
                Tribe::Roman => 1.030,
                Tribe::Teuton => 1.020,
                Tribe::Gaul => 1.025,
                _ => 1.020,
            };
            return tribe_factor.powf(wall.level as f64);
        }
        1.0
    }

    pub fn get_buildings_durability(&self) -> f64 {
        match self.get_building_by_name(BuildingName::StonemansionLodge) {
            Some(b) => 1.0 + b.level as f64 * 0.1,
            None => 1.0,
        }
    }

    /// Updates the village stocks based on the time elapsed since the last update
    /// It should be called whenever the village is loaded from the DB.
    pub fn update_resources(&mut self) {
        let now = Utc::now();
        let time_elapsed_secs = (now - self.updated_at).num_seconds();

        if time_elapsed_secs <= 0 {
            self.updated_at = now;
            return;
        }

        // Effective hourly production
        // update effective production apllying bonuses and upkeep
        self.production.calculate_effective_production();
        let effective_prod = &self.production.effective;

        // Calculates per-second production rates
        // Actual effective production is per hour
        let lumber_per_sec = (effective_prod.lumber as f64) / 3600.0;
        let clay_per_sec = (effective_prod.clay as f64) / 3600.0;
        let iron_per_sec = (effective_prod.iron as f64) / 3600.0;
        let crop_per_sec = (effective_prod.crop as f64) / 3600.0;

        let elapsed_f64 = time_elapsed_secs as f64;

        // Calculates resources after elapsed time, capping at storage capacity
        let new_lumber = (self.stocks.lumber as f64 + (elapsed_f64 * lumber_per_sec))
            .min(self.stocks.warehouse_capacity as f64);

        let new_clay = (self.stocks.clay as f64 + (elapsed_f64 * clay_per_sec))
            .min(self.stocks.warehouse_capacity as f64);

        let new_iron = (self.stocks.iron as f64 + (elapsed_f64 * iron_per_sec))
            .min(self.stocks.warehouse_capacity as f64);

        let new_crop = (self.stocks.crop as f64 + (elapsed_f64 * crop_per_sec))
            .min(self.stocks.granary_capacity as f64);

        // New stocks
        self.stocks.lumber = new_lumber.max(0.0) as u32;
        self.stocks.clay = new_clay.max(0.0) as u32;
        self.stocks.iron = new_iron.max(0.0) as u32;
        self.stocks.crop = new_crop as i64; // crop can be negative due to upkeep

        self.updated_at = now;
    }

    // Updates the village stats (population, production, bonuses from buildings and oases, etc).
    fn update_state(&mut self) {
        self.population = 2;
        self.production = Default::default();

        // reset the stocks capacities because we're going to recalculate them
        self.stocks.warehouse_capacity = 0;
        self.stocks.granary_capacity = 0;

        // data from infrastructures
        for b in self.buildings.iter() {
            self.population += b.building.cost().upkeep;

            match b.building.name {
                BuildingName::Woodcutter => self.production.lumber += b.building.value,
                BuildingName::ClayPit => self.production.clay += b.building.value,
                BuildingName::IronMine => self.production.iron += b.building.value,
                BuildingName::Cropland => self.production.crop += b.building.value,
                BuildingName::Sawmill => self.production.bonus.lumber += b.building.value as u8,
                BuildingName::Brickyard => self.production.bonus.clay += b.building.value as u8,
                BuildingName::IronFoundry => self.production.bonus.iron += b.building.value as u8,
                BuildingName::GrainMill => self.production.bonus.crop += b.building.value as u8,
                BuildingName::Bakery => self.production.bonus.crop += b.building.value as u8,
                BuildingName::Warehouse => self.stocks.warehouse_capacity += b.building.value,
                BuildingName::Granary => self.stocks.granary_capacity += b.building.value,
                BuildingName::GreatWarehouse => self.stocks.warehouse_capacity += b.building.value,
                BuildingName::GreatGranary => self.stocks.granary_capacity += b.building.value,
                _ => continue,
            }
        }

        // set default stocks capacities if no warehouse/granary present
        if self.stocks.granary_capacity == 0 {
            self.stocks.granary_capacity = VillageStocks::default().granary_capacity;
        }
        if self.stocks.warehouse_capacity == 0 {
            self.stocks.warehouse_capacity = VillageStocks::default().warehouse_capacity;
        }

        // population upkeep
        self.production.upkeep += self.population;

        // oases production bonuses
        for o in self.oases.clone() {
            let oasis_bonus = o.bonus();
            self.production.bonus.add(&oasis_bonus)
        }

        // armies upkeep
        self.production.upkeep += match self.army.clone() {
            Some(army) => army.upkeep(),
            None => 0,
        };
        for a in self.reinforcements.iter() {
            self.production.upkeep += a.upkeep();
        }

        self.update_resources();
    }

    fn init_village_buildings(&mut self, valley: &Valley) -> Result<()> {
        let topology = valley.topology.clone();

        // Default resources level 0
        for _ in 0..topology.lumber() {
            let slot_id = self.buildings.len() as u8 + 1;
            let building = Building::new(BuildingName::Woodcutter).at_level(0)?;
            self.buildings
                .append(&mut vec![VillageBuilding { slot_id, building }]);
        }

        for _ in 0..topology.clay() {
            let slot_id = self.buildings.len() as u8 + 1;
            let building = Building::new(BuildingName::ClayPit).at_level(0)?;
            self.buildings
                .append(&mut vec![VillageBuilding { slot_id, building }]);
        }

        for _ in 0..topology.iron() {
            let slot_id = self.buildings.len() as u8 + 1;
            let building = Building::new(BuildingName::IronMine).at_level(0)?;
            self.buildings
                .append(&mut vec![VillageBuilding { slot_id, building }]);
        }

        for _ in 0..topology.crop() {
            let slot_id = self.buildings.len() as u8 + 1;
            let building = Building::new(BuildingName::Cropland).at_level(0)?;
            self.buildings
                .append(&mut vec![VillageBuilding { slot_id, building }]);
        }

        // Main building level 1
        let main_building = Building::new(BuildingName::MainBuilding);
        let slot_id: u8 = self.buildings.len() as u8 + 1;
        self.buildings.append(&mut vec![VillageBuilding {
            slot_id,
            building: main_building,
        }]);

        Ok(())
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct VillageStocks {
    pub warehouse_capacity: u32,
    pub granary_capacity: u32,
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: i64,
}

impl VillageStocks {
    /// Returns the total storage capacity (warehouse + granary)
    pub fn total_capacity(&self) -> u32 {
        self.warehouse_capacity + self.granary_capacity
    }

    /// Returns the currently stored resources as ResourceGroup
    pub fn stored_resources(&self) -> ResourceGroup {
        ResourceGroup::new(self.lumber, self.clay, self.iron, self.crop.max(0) as u32)
    }
}

impl Default for VillageStocks {
    fn default() -> Self {
        Self {
            warehouse_capacity: 800, // Base capacity
            granary_capacity: 800,   // Base capacity
            lumber: 0,
            clay: 0,
            iron: 0,
            crop: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::game::{
        models::buildings::BuildingName,
        test_factories::{village_factory, VillageFactoryOptions},
    };

    #[test]
    fn test_new_village() {
        let v = village_factory(VillageFactoryOptions {
            ..Default::default()
        });

        // resource fields and main building
        assert_eq!(v.buildings.len(), 19, "number of total buildings");
        let mut woodcutter = 0;
        let mut clay_pit = 0;
        let mut iron_mine = 0;
        let mut cropland = 0;
        let mut main_building = false;

        for b in v.buildings {
            match b.building.name {
                BuildingName::Woodcutter => woodcutter += 1,
                BuildingName::ClayPit => clay_pit += 1,
                BuildingName::IronMine => iron_mine += 1,
                BuildingName::Cropland => cropland += 1,
                BuildingName::MainBuilding => main_building = true,
                _ => (),
            }
        }
        assert_eq!(woodcutter, 4, "woodcutter fields");
        assert_eq!(clay_pit, 4, "clay pit fields");
        assert_eq!(iron_mine, 4, "iron mine fields");
        assert_eq!(cropland, 6, "cropland fields");
        assert!(main_building, "main building is not present");

        // production
        assert_eq!(v.production.lumber, 8, "lumber production");
        assert_eq!(v.production.clay, 8, "clay production");
        assert_eq!(v.production.iron, 8, "iron production");
        assert_eq!(v.production.crop, 12, "crop production");
        assert_eq!(v.production.upkeep, 4, "upkeep");

        // population
        assert_eq!(v.population, 4, "population");

        // stocks
        assert_eq!(v.stocks.warehouse_capacity, 800, "stock warehouse");
        assert_eq!(v.stocks.granary_capacity, 800, "stock granary");
    }
}
