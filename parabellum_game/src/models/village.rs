use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_core::GameError;
use parabellum_types::{
    army::UnitName,
    buildings::{BuildingGroup, BuildingName},
    common::{Player, ResourceGroup},
    map::Position,
    tribe::Tribe,
};

use crate::battle::BattleReport;

use super::{
    army::Army,
    buildings::Building,
    map::{Oasis, Valley},
    smithy::SmithyUpgrades,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VillageBuilding {
    pub slot_id: u8,
    pub building: Building,
}

pub type AcademyResearch = [bool; 10];

// TODO: add standalone rally point? Not yet
// TODO: add standalone wall? Not yet
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
    pub academy_research: AcademyResearch,
    pub total_merchants: u8,
    pub busy_merchants: u8,
    pub updated_at: DateTime<Utc>,
}

impl Village {
    pub fn new(
        name: String,
        valley: &Valley,
        player: &Player,
        is_capital: bool,
        world_size: i32,
        server_speed: i8,
    ) -> Self {
        let position = valley.position.clone();
        let village_id = position.to_id(world_size);

        let production: VillageProduction = Default::default();
        let smithy = [0, 0, 0, 0, 0, 0, 0, 0];
        let academy_research = [false; 10];

        let mut village = Self {
            id: village_id,
            name,
            position,
            player_id: player.id,
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
            academy_research,
            total_merchants: 0,
            busy_merchants: 0,
            stocks: Default::default(),
            updated_at: Utc::now(),
        };

        // FIXME: either fix the method return value or this method one.
        village
            .init_village_buildings(valley, server_speed)
            .unwrap();
        village.update_merchants_count();
        village.update_state();
        village
    }

    /// Build a new building on a given slot.
    pub fn add_building_at_slot(
        &mut self,
        building: Building,
        slot_id: u8,
    ) -> Result<(), GameError> {
        self.buildings.push(VillageBuilding { slot_id, building });
        self.update_state();

        Ok(())
    }

    pub fn set_building_level_at_slot(
        &mut self,
        slot_id: u8,
        level: u8,
        server_speed: i8,
    ) -> Result<(), GameError> {
        let idx = self
            .buildings
            .iter()
            .position(|b| b.slot_id == slot_id)
            .ok_or_else(|| GameError::EmptySlot { slot_id })?;

        let building = self.buildings[idx].building.clone();
        let _ = std::mem::replace(
            &mut self.buildings[idx],
            VillageBuilding {
                slot_id,
                building: building.at_level(level, server_speed)?,
            },
        );
        self.update_state();
        Ok(())
    }

    /// Removes a building from a given slot, except for resource fields because they can just go to level 0.
    pub fn remove_building_at_slot(
        &mut self,
        slot_id: u8,
        server_speed: i8,
    ) -> Result<(), GameError> {
        if slot_id >= 1 && slot_id <= 18 {
            return self.set_building_level_at_slot(slot_id, 0, server_speed);
        }

        self.buildings.retain(|vb| vb.slot_id != slot_id);
        self.update_state();
        Ok(())
    }

    /// Returns a building in the village on a given slot. Returns None if not present.
    pub fn get_building_by_slot_id(&self, slot_id: u8) -> Option<VillageBuilding> {
        self.buildings
            .iter()
            .find(|&x| x.slot_id == slot_id)
            .cloned()
    }

    /// Returns a building in the village. Returns None if not present.
    /// In case of multiple buildings of same type, it returns the highest level one.
    pub fn get_building_by_name(&self, name: BuildingName) -> Option<VillageBuilding> {
        if let Some(village_building) = self
            .buildings
            .iter()
            .filter(|&x| x.building.name == name)
            .cloned()
            .max_by(|x, y| x.building.level.cmp(&y.building.level))
        {
            return Some(village_building);
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

        buildings
            .choose_multiple(&mut rng, count)
            .cloned()
            .collect::<Vec<Building>>()
    }

    /// Returns either the Palace or Residence, if any.
    pub fn get_palace_or_residence(&self) -> Option<(Building, BuildingName)> {
        if let Some(palace) = self.get_building_by_name(BuildingName::Palace) {
            return Some((palace.building, BuildingName::Palace));
        }
        if let Some(residence) = self.get_building_by_name(BuildingName::Residence) {
            return Some((residence.building, BuildingName::Residence));
        }
        None
    }

    /// Returns the village wall, if any, according to the actual tribe.
    pub fn get_wall(&self) -> Option<Building> {
        match self.tribe {
            Tribe::Roman => self.get_building_by_name(BuildingName::CityWall),
            Tribe::Teuton => self.get_building_by_name(BuildingName::EarthWall),
            Tribe::Gaul => self.get_building_by_name(BuildingName::Palisade),
            _ => None,
        }
        .map(|vb| vb.building)
    }

    /// Get defense bonus from wall, if any.
    pub fn get_wall_defense_bonus(&self) -> f64 {
        if let Some(wall) = self.get_wall() {
            return self.tribe.get_wall_factor().powf(wall.level as f64);
        }
        1.0
    }

    /// Get buildings durability, considering the level of StonemansionLodge, if any.
    pub fn get_buildings_durability(&self) -> f64 {
        match self.get_building_by_name(BuildingName::StonemansionLodge) {
            Some(b) => 1.0 + b.building.level as f64 * 0.1,
            None => 1.0,
        }
    }

    /// Applies combat losses to the village's army and reinforcements based on the battle report.
    pub fn apply_battle_losses(&mut self, report: &BattleReport) {
        let mut final_home_army = None;
        if let Some(defender_report) = &report.defender {
            if let Some(mut home_army) = self.army.take() {
                home_army.update_units(&defender_report.survivors);
                if home_army.immensity() > 0 {
                    final_home_army = Some(home_army);
                }
            }
        } else if self.army.is_some() {
            // If no defender report but army exists, it means total loss
            final_home_army = self.army.take();
        }
        self.army = final_home_army;

        for report in &report.reinforcements {
            if let Some(index) = self
                .reinforcements
                .iter()
                .position(|r| r.id == report.army_before.id)
            {
                let mut army = self.reinforcements.remove(index);
                army.update_units(&report.survivors);
                if army.immensity() > 0 {
                    self.reinforcements.insert(index, army.clone());
                }
            }
        }
    }

    /// Applies building damages from the battle report to the village.
    pub fn apply_building_damages(
        &mut self,
        report: &BattleReport,
        server_speed: i8,
    ) -> Result<(), GameError> {
        // Wall damage
        if let Some(wall_damage) = &report.wall_damage {
            if wall_damage.level_after < wall_damage.level_before {
                if let Some(wall_building) = self.get_building_by_name(wall_damage.name.clone()) {
                    // Find the VillageBuilding for the wall
                    if let Some(vb) = self
                        .buildings
                        .iter_mut()
                        .find(|vb| vb.building.name == wall_damage.name)
                    {
                        vb.building = wall_building
                            .building
                            .at_level(wall_damage.level_after, server_speed)?;
                    }
                }
            }
        }

        // Catapult damages to other buildings
        for damage_report in &report.catapult_damage {
            if damage_report.level_after < damage_report.level_before {
                if let Some(target_building) = self.get_building_by_name(damage_report.name.clone())
                {
                    // Trova il VillageBuilding per lo slot_id
                    // Nota: Questo assume che tu abbia memorizzato lo slot_id
                    // nel report o che tu possa recuperarlo dal nome.
                    // Se hai più edifici dello stesso tipo, devi identificare
                    // quello specifico bersagliato.
                    // Qui assumo che ci sia uno slot_id fittizio 0 per semplicità.
                    let slot_id_da_trovare = self
                        .buildings
                        .iter()
                        .find(|vb| vb.building.name == damage_report.name)
                        .map_or(0, |vb| vb.slot_id); // Trova il vero slot_id

                    if let Some(vb) = self
                        .buildings
                        .iter_mut()
                        .find(|vb| vb.slot_id == slot_id_da_trovare)
                    {
                        vb.building = target_building
                            .building
                            .at_level(damage_report.level_after, server_speed)?;

                        if damage_report.level_after == 0
                            && vb.building.group != BuildingGroup::Resources
                        {
                            self.buildings.retain(|b| b.slot_id != slot_id_da_trovare);
                        }
                    }
                }
            }
        }
        self.update_state();
        Ok(())
    }

    /// Returns available merchants.
    pub fn get_available_merchants(&self) -> u8 {
        self.total_merchants.saturating_sub(self.busy_merchants)
    }

    /// Updates the village state (production, upkeep, etc...).
    pub fn update_state(&mut self) {
        self.population = 2;
        self.production = Default::default();

        // reset the stocks capacities because we're going to recalculate them
        self.stocks.warehouse_capacity = 0;
        self.stocks.granary_capacity = 0;

        // data from infrastructures
        for b in self.buildings.iter() {
            self.population += b.building.population;

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
        let default_socks = VillageStocks::default();
        if self.stocks.granary_capacity == 0 {
            self.stocks.granary_capacity = default_socks.granary_capacity;
        }
        if self.stocks.warehouse_capacity == 0 {
            self.stocks.warehouse_capacity = default_socks.warehouse_capacity;
        }

        // population upkeep
        self.production.upkeep += self.population;

        // oases production bonuses
        for o in self.oases.clone() {
            let oasis_bonus = o.bonus();
            self.production.bonus.add(&oasis_bonus)
        }

        // armies upkeep
        self.production.upkeep += self.army.clone().map_or(0, |a| a.upkeep());
        for a in self.reinforcements.iter() {
            self.production.upkeep += a.upkeep();
        }

        // update internal data
        self.production.calculate_effective_production();
        self.update_merchants_count();
        self.update_resources();
    }

    /// Marks a unit name as researched in the academy.
    pub fn research_academy(&mut self, unit: UnitName) -> Result<(), GameError> {
        if let Some(idx) = self.tribe.get_unit_idx_by_name(&unit) {
            self.academy_research[idx] = true;
        }

        Ok(())
    }

    /// Upgrades smithy level for unit name.
    pub fn upgrade_smithy(&mut self, unit: UnitName) -> Result<(), GameError> {
        if let Some(idx) = self.tribe.get_unit_idx_by_name(&unit) {
            if self.smithy[idx] < 20 {
                self.smithy[idx] += 1;
            }
        }

        Ok(())
    }

    /// Sets total merchants count based on Marketplace level.
    fn update_merchants_count(&mut self) {
        if let Some(marketplace) = self.get_building_by_name(BuildingName::Marketplace) {
            self.total_merchants = marketplace.building.level;
        } else {
            self.total_merchants = 0;
        }
    }

    /// Updates the village stocks based on the time elapsed since the last update
    /// It should be called whenever the village is loaded from the DB.
    fn update_resources(&mut self) {
        let now = Utc::now();
        let time_elapsed = (now - self.updated_at).num_seconds() as f64;

        if time_elapsed <= 0.0 {
            self.updated_at = now;
            return;
        }

        let (lumber_delta, clay_delta, iron_delta, crop_delta) =
            self.production.calculate_production_deltas(time_elapsed);

        self.stocks.lumber = (self.stocks.lumber as f64 + lumber_delta)
            .min(self.stocks.warehouse_capacity as f64)
            .max(0.0)
            .floor() as u32;
        self.stocks.clay = (self.stocks.clay as f64 + clay_delta)
            .min(self.stocks.warehouse_capacity as f64)
            .max(0.0)
            .floor() as u32;
        self.stocks.iron = (self.stocks.iron as f64 + iron_delta)
            .min(self.stocks.warehouse_capacity as f64)
            .max(0.0)
            .floor() as u32;

        let new_crop =
            (self.stocks.crop as f64 + crop_delta).min(self.stocks.granary_capacity as f64);

        // Handle starvation (if crop goes negative)
        if new_crop < 0.0 {
            // TODO: Implement starvation logic (kill troops, etc.)
            // For now, just cap stock at 0
            self.stocks.crop = 0;
        } else {
            self.stocks.crop = new_crop.floor() as i64;
        }

        self.updated_at = now;
    }

    fn init_village_buildings(
        &mut self,
        valley: &Valley,
        server_speed: i8,
    ) -> Result<(), GameError> {
        let topology = valley.topology.clone();

        // Default resources level 0
        for _ in 0..topology.lumber() {
            let slot_id = self.buildings.len() as u8 + 1;
            let building =
                Building::new(BuildingName::Woodcutter, server_speed).at_level(0, server_speed)?;
            self.buildings
                .append(&mut vec![VillageBuilding { slot_id, building }]);
        }

        for _ in 0..topology.clay() {
            let slot_id = self.buildings.len() as u8 + 1;
            let building =
                Building::new(BuildingName::ClayPit, server_speed).at_level(0, server_speed)?;
            self.buildings
                .append(&mut vec![VillageBuilding { slot_id, building }]);
        }

        for _ in 0..topology.iron() {
            let slot_id = self.buildings.len() as u8 + 1;
            let building =
                Building::new(BuildingName::IronMine, server_speed).at_level(0, server_speed)?;
            self.buildings
                .append(&mut vec![VillageBuilding { slot_id, building }]);
        }

        for _ in 0..topology.crop() {
            let slot_id = self.buildings.len() as u8 + 1;
            let building =
                Building::new(BuildingName::Cropland, server_speed).at_level(0, server_speed)?;
            self.buildings
                .append(&mut vec![VillageBuilding { slot_id, building }]);
        }

        // Main building level 1
        let main_building = Building::new(BuildingName::MainBuilding, server_speed);
        let slot_id: u8 = self.buildings.len() as u8 + 1;
        self.buildings.append(&mut vec![VillageBuilding {
            slot_id,
            building: main_building,
        }]);

        self.update_state();

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

    pub fn calculate_production_deltas(&self, time_elapsed_secs: f64) -> (f64, f64, f64, f64) {
        let effective_prod = &self.effective;

        let lumber_per_sec = (effective_prod.lumber as f64) / 3600.0;
        let clay_per_sec = (effective_prod.clay as f64) / 3600.0;
        let iron_per_sec = (effective_prod.iron as f64) / 3600.0;
        let crop_per_sec = (effective_prod.crop as f64) / 3600.0;

        let lumber_delta = time_elapsed_secs * lumber_per_sec;
        let clay_delta = time_elapsed_secs * clay_per_sec;
        let iron_delta = time_elapsed_secs * iron_per_sec;
        let crop_delta = time_elapsed_secs * crop_per_sec;

        (lumber_delta, clay_delta, iron_delta, crop_delta)
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

    /// Stores resources into the village stocks, capping at storage capacity.
    pub fn store_resources(&mut self, resources: ResourceGroup) {
        self.lumber = (self.lumber + resources.0).min(self.warehouse_capacity);
        self.clay = (self.clay + resources.1).min(self.warehouse_capacity);
        self.iron = (self.iron + resources.2).min(self.warehouse_capacity);
        self.crop = (self.crop + resources.3 as i64).min(self.granary_capacity as i64);
    }

    /// Checks if given resources are present in stocks.
    pub fn check_resources(&self, resources: &ResourceGroup) -> bool {
        self.lumber >= resources.0
            && self.clay >= resources.1
            && self.iron >= resources.2
            && self.crop >= resources.3 as i64
    }

    /// Removes resources from the village stocks, ensuring they don't go negative.
    pub fn remove_resources(&mut self, resources: &ResourceGroup) {
        self.lumber = (self.lumber as i64 - resources.0 as i64).max(0) as u32;
        self.clay = (self.clay as i64 - resources.1 as i64).max(0) as u32;
        self.iron = (self.iron as i64 - resources.2 as i64).max(0) as u32;
        self.crop -= resources.3 as i64; // crop can go negative
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
    use crate::{
        models::village::VillageStocks,
        test_utils::{
            PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions, player_factory,
            valley_factory, village_factory,
        },
    };
    use chrono::{Duration, Utc};
    use parabellum_types::{buildings::BuildingName, map::ValleyTopology, tribe::Tribe};

    #[test]
    fn test_new_village() {
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        let valley = valley_factory(ValleyFactoryOptions {
            topology: Some(ValleyTopology(4, 4, 4, 6)),
            ..Default::default()
        });

        let v = village_factory(VillageFactoryOptions {
            player: Some(player),
            valley: Some(valley),
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

        // production (level 0 fields + main building level 1)
        // Level 0 fields produce 2 units/hour each (Travian T3 logic)
        // 4 * 2 = 8 lumber
        // 4 * 2 = 8 clay
        // 4 * 2 = 8 iron
        // 6 * 2 = 12 crop
        assert_eq!(v.production.lumber, 8, "lumber production");
        assert_eq!(v.production.clay, 8, "clay production");
        assert_eq!(v.production.iron, 8, "iron production");
        assert_eq!(v.production.crop, 12, "crop production");

        // Upkeep: 2 (base) + 2 (main building) = 4
        assert_eq!(v.production.upkeep, 4, "upkeep");

        // Population: 2 (base) + 2 (main building) = 4
        assert_eq!(v.population, 4, "population");

        // Stocks
        assert_eq!(v.stocks.warehouse_capacity, 800, "stock warehouse");
        assert_eq!(v.stocks.granary_capacity, 800, "stock granary");

        // Effective production
        // 12 crop - 4 upkeep = 8 effective crop
        assert_eq!(v.production.effective.crop, 8, "effective crop production");
    }

    #[test]
    fn test_update_resources() {
        let mut v = village_factory(VillageFactoryOptions {
            ..Default::default()
        });

        // Default production for 4-4-4-6 village (level 0 fields)
        // Lumber: 8/h -> 0.00222.../s
        // Clay:   8/h -> 0.00222.../s
        // Iron:   8/h -> 0.00222.../s
        // Crop:   12/h - 4 upkeep = 8/h -> 0.00222.../s

        let effective_prod_per_sec = 8.0 / 3600.0;

        // Simulate 1 hour (3600 seconds) passing
        let one_hour_ago = Utc::now() - Duration::seconds(3600);
        v.updated_at = one_hour_ago;
        v.stocks = VillageStocks {
            // Start with 0 resources
            lumber: 0,
            clay: 0,
            iron: 0,
            crop: 0,
            ..v.stocks
        };

        v.update_state(); // This calls update_resources internally

        let expected_resources = (effective_prod_per_sec as f64 * 3600.0).floor() as u32; // Should be 8

        assert_eq!(expected_resources, 8);
        assert_eq!(v.stocks.lumber, 8, "Lumber should be 8 after 1 hour");
        assert_eq!(v.stocks.clay, 8, "Clay should be 8 after 1 hour");
        assert_eq!(v.stocks.iron, 8, "Iron should be 8 after 1 hour");
        assert_eq!(v.stocks.crop, 8, "Crop should be 8 after 1 hour");

        // Simulate 100 hours (over capacity)
        let long_time_ago = Utc::now() - Duration::hours(100);
        v.updated_at = long_time_ago;
        v.stocks = VillageStocks {
            // Start with 0 resources
            lumber: 0,
            clay: 0,
            iron: 0,
            crop: 0,
            ..v.stocks
        };

        v.update_state();

        // Resources should be capped at base capacity (800)
        assert_eq!(
            v.stocks.lumber, 800,
            "Lumber should be capped at warehouse capacity"
        );
        assert_eq!(
            v.stocks.clay, 800,
            "Clay should be capped at warehouse capacity"
        );
        assert_eq!(
            v.stocks.iron, 800,
            "Iron should be capped at warehouse capacity"
        );
        assert_eq!(
            v.stocks.crop, 800,
            "Crop should be capped at granary capacity"
        );
    }

    #[test]
    fn test_cumulative_population_and_upkeep_on_upgrade() {
        let server_speed = 1;
        let mut v = village_factory(VillageFactoryOptions {
            server_speed: Some(server_speed),
            ..Default::default()
        });

        let mb_slot_id = 19;
        let mb_l1 = v.get_building_by_slot_id(mb_slot_id).unwrap().building;

        // Initial population upkeep: 2 (base) + 2 (MB L1) = 4
        assert_eq!(mb_l1.level, 1);
        assert_eq!(
            mb_l1.population, 2,
            "Main Building L1 population should be 2"
        );
        assert_eq!(v.population, 4, "Initial village population should be 4");
        assert_eq!(v.production.upkeep, 4, "Initial village upkeep should be 4");

        v.set_building_level_at_slot(mb_slot_id, 2, server_speed)
            .unwrap();
        v.update_state();

        let mb_l2 = v.get_building_by_slot_id(mb_slot_id).unwrap().building;

        assert_eq!(mb_l2.level, 2);
        assert_eq!(
            mb_l2.population, 3,
            "Main Building L2 cumulative population should be 3 (2+1)"
        );

        assert_eq!(
            v.population, 5,
            "Village population with MainBuilding L2 should be 5"
        );
        assert_eq!(
            v.production.upkeep, 5,
            "Village upkeep with MainBuilding L2 should be 5"
        );

        v.set_building_level_at_slot(mb_slot_id, 3, server_speed)
            .unwrap();
        v.update_state();

        let mb_l3 = v.get_building_by_slot_id(mb_slot_id).unwrap().building;

        assert_eq!(mb_l3.level, 3);
        assert_eq!(
            mb_l3.population, 4,
            "Main Building L3 population should be 4 (2+1+1)"
        );

        assert_eq!(
            v.population, 6,
            "Village population with Main Building L3 should be 6"
        );
        assert_eq!(
            v.production.upkeep, 6,
            "Village upkeep with Main Building L3 upgrade should be 6"
        );
    }
}
