use chrono::{DateTime, Utc};
use parabellum_types::army::{Unit, UnitGroup};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_types::errors::GameError;
use parabellum_types::{
    army::UnitName,
    buildings::{BuildingGroup, BuildingName, BuildingRequirement},
    common::{Player, ResourceGroup},
    map::Position,
    tribe::Tribe,
};

use crate::{
    battle::BattleReport,
    models::{
        buildings::{BuildingConstraint, get_building_data},
        smithy::smithy_upgrade_cost_for_unit,
    },
};

use super::{
    army::Army,
    buildings::Building,
    map::{Oasis, Valley},
    smithy::SmithyUpgrades,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageBuilding {
    pub slot_id: u8,
    pub building: Building,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AcademyResearch {
    researches: [bool; 10],
}

impl AcademyResearch {
    pub fn get(&self, idx: usize) -> bool {
        self.researches[idx]
    }

    pub fn set(&mut self, idx: usize, value: bool) {
        self.researches[idx] = value;
    }
}

impl Default for AcademyResearch {
    fn default() -> Self {
        Self {
            researches: [
                true, false, false, false, false, false, false, false, false, true,
            ],
        }
    }
}

const RESOURCE_FIELDS_LAST_SLOT: u8 = 18;
const MAIN_BUILDING_SLOT_ID: u8 = 19;
const RALLY_POINT_SLOT_ID: u8 = 39;
const WALL_SLOT_ID: u8 = 40;
const MAX_VILLAGE_SLOT_ID: u8 = 40;
const COMMON_BUILDINGS: [BuildingName; 32] = [
    BuildingName::Sawmill,
    BuildingName::Brickyard,
    BuildingName::IronFoundry,
    BuildingName::GrainMill,
    BuildingName::Bakery,
    BuildingName::Warehouse,
    BuildingName::Granary,
    BuildingName::Smithy,
    BuildingName::TournamentSquare,
    BuildingName::Marketplace,
    BuildingName::Embassy,
    BuildingName::Barracks,
    BuildingName::Stable,
    BuildingName::Workshop,
    BuildingName::Academy,
    BuildingName::Cranny,
    BuildingName::TownHall,
    BuildingName::Residence,
    BuildingName::Palace,
    BuildingName::Treasury,
    BuildingName::TradeOffice,
    BuildingName::GreatBarracks,
    BuildingName::GreatStable,
    BuildingName::StonemansionLodge,
    BuildingName::Brewery,
    BuildingName::Trapper,
    BuildingName::HeroMansion,
    BuildingName::GreatWarehouse,
    BuildingName::GreatGranary,
    BuildingName::WonderOfTheWorld,
    BuildingName::HorseDrinkingTrough,
    BuildingName::GreatWorkshop,
];

// TODO: add standalone rally point? Not yet
// TODO: add standalone wall? Not yet
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Village {
    pub id: u32,
    pub name: String,
    pub player_id: Uuid,
    pub position: Position,
    pub tribe: Tribe,
    pub oases: Vec<Oasis>,
    pub population: u32,
    buildings: Vec<VillageBuilding>,
    army: Option<Army>,
    reinforcements: Vec<Army>,
    smithy: SmithyUpgrades,
    stocks: VillageStocks,
    academy_research: AcademyResearch,
    deployed_armies: Vec<Army>,
    loyalty: u8,
    pub production: VillageProduction,
    pub is_capital: bool,
    pub total_merchants: u8,
    pub busy_merchants: u8,
    pub updated_at: DateTime<Utc>,
}

impl Village {
    /// Returns a new village instance.
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
        let academy_research = AcademyResearch::default();

        let mut village = Self {
            id: village_id,
            name,
            position,
            player_id: player.id,
            tribe: player.tribe.clone(),
            buildings: vec![],
            oases: vec![],
            population: 0,
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

    /// Constructor for re-hydrating a Village from persistence (database).
    #[allow(clippy::too_many_arguments)]
    pub fn from_persistence(
        id: u32,
        name: String,
        player_id: Uuid,
        position: Position,
        tribe: Tribe,
        buildings: Vec<VillageBuilding>,
        oases: Vec<Oasis>,
        population: u32,
        army: Option<Army>,
        reinforcements: Vec<Army>,
        deployed_armies: Vec<Army>,
        loyalty: u8,
        production: VillageProduction,
        is_capital: bool,
        smithy: SmithyUpgrades,
        stocks: VillageStocks,
        academy_research: AcademyResearch,
        updated_at: DateTime<Utc>,
    ) -> Self {
        let mut village = Self {
            id,
            name,
            player_id,
            position,
            tribe,
            buildings,
            oases,
            population,
            army,
            reinforcements,
            deployed_armies,
            loyalty,
            production,
            is_capital,
            smithy,
            stocks,
            academy_research,
            total_merchants: 0,
            busy_merchants: 0,
            updated_at,
        };

        village.update_state();
        village
    }

    /// Prepares for training a new unit, applies validations and withdraws resources. Returns training times.
    pub fn init_unit_training(
        &mut self,
        unit_idx: u8,
        building_name: &BuildingName,
        quantity: i32,
        server_speed: i8,
    ) -> Result<(u8, UnitName, u32), GameError> {
        let unit = self
            .tribe
            .units()
            .get(unit_idx as usize)
            .ok_or(GameError::InvalidUnitIndex(unit_idx))?
            .to_owned();

        if !self.academy_research.get(unit_idx as usize) && unit.research_cost.time > 0 {
            return Err(GameError::UnitNotResearched(unit.name.clone()));
        }

        if !unit.buildings.contains(building_name) {
            return Err(GameError::InvalidTrainingBuilding(
                building_name.clone(),
                unit.name.clone(),
            ));
        }

        let building = self
            .get_building_by_name(&building_name.clone())
            .ok_or_else(|| GameError::BuildingRequirementsNotMet {
                building: building_name.clone(),
                level: 1,
            })?;

        let cost_per_unit = &(unit.cost);
        let total_cost = &(cost_per_unit.resources.clone() * quantity.into());
        self.deduct_resources(total_cost)?;

        let training_time_bonus_perc = building.building.value as f64 / 1000.0; // Es: 100 -> 1.0, 90 -> 0.9
        let base_time_per_unit = cost_per_unit.time as f64 / server_speed as f64;

        let time_per_unit = (base_time_per_unit * training_time_bonus_perc)
            .floor()
            .max(1.0) as u32;

        Ok((building.slot_id, unit.name.clone(), time_per_unit))
    }

    /// Prepares for adding a new building, applies validations and withdraws resources. Returns construction times.
    pub fn init_building_construction(
        &mut self,
        slot_id: u8,
        name: BuildingName,
        server_speed: i8,
    ) -> Result<u32, GameError> {
        if self.buildings.len() == 40 {
            return Err(GameError::VillageSlotsFull);
        }
        if self.get_building_by_slot_id(slot_id).is_some() {
            return Err(GameError::SlotOccupied { slot_id });
        }

        let building = Building::new(name, server_speed);
        self.validate_building_construction(&building)?;

        self.deduct_resources(&building.cost().resources)?;
        let mb_level = self.main_building_level();
        Ok(building.calculate_build_time_secs(&server_speed, &mb_level))
    }

    /// Prepares for starting a research in academy, applies validations and withdraws resources. Returns research time.
    pub fn init_academy_research(
        &mut self,
        unit: &UnitName,
        server_speed: i8,
    ) -> Result<u32, GameError> {
        let unit_idx = self.tribe.get_unit_idx_by_name(unit).unwrap();

        if self.academy_research().get(unit_idx) {
            return Err(GameError::UnitAlreadyResearched(unit.clone()));
        }

        let tribe = self.tribe.clone();
        let unit_data = tribe
            .get_unit_by_name(unit)
            .ok_or_else(|| GameError::UnitNotFound(unit.clone()))?;
        self.validate_building_requirements(unit_data.requirements)?;

        let research_cost = &unit_data.research_cost;
        self.deduct_resources(&research_cost.resources)?;
        let research_time_secs = (research_cost.time as f64 / server_speed as f64).floor() as u32;

        Ok(research_time_secs)
    }

    /// Prepares for starting a research in smithy, applies validations and withdraws resources. Returns research time.
    pub fn init_smithy_research(
        &mut self,
        unit_name: &UnitName,
        server_speed: i8,
    ) -> Result<u32, GameError> {
        let unit_idx = self.tribe.get_unit_idx_by_name(unit_name).unwrap();
        let current_level = self.smithy()[unit_idx];

        let unit = self
            .tribe
            .units()
            .get(unit_idx)
            .ok_or(GameError::InvalidUnitIndex(unit_idx as u8))?;

        for req in unit.get_requirements() {
            if !self.buildings.iter().any(|b| b.building.name == req.0) {
                return Err(GameError::BuildingRequirementsNotMet {
                    building: req.0.clone(),
                    level: req.1,
                });
            }
        }

        if unit.research_cost.time > 0 && !self.academy_research().get(unit_idx as usize) {
            return Err(GameError::UnitNotResearched(unit_name.clone()));
        }

        let research_cost = smithy_upgrade_cost_for_unit(unit_name, current_level)?;
        self.deduct_resources(&research_cost.resources)?;
        let research_time_secs = (research_cost.time as f64 / server_speed as f64).floor() as u32;

        Ok(research_time_secs)
    }

    /// Returns a reference to the smithy upgrades for persistence (serialization).
    pub fn smithy(&self) -> &SmithyUpgrades {
        &self.smithy
    }

    /// Returns a reference to the academy research for persistence (serialization).
    pub fn academy_research(&self) -> &AcademyResearch {
        &self.academy_research
    }

    /// Returns a reference to the village stocks for persistence (serialization).
    /// This should primarily be used by the repository layer.
    pub fn stocks(&self) -> &VillageStocks {
        &self.stocks
    }

    /// Returns a reference to the village buildings for persistence (serialization).
    /// This should primarily be used by the repository layer.
    pub fn buildings(&self) -> &Vec<VillageBuilding> {
        &self.buildings
    }

    /// Checks if the village has enough resources.
    pub fn has_enough_resources(&self, cost: &ResourceGroup) -> bool {
        self.stocks.has_availability(cost)
    }

    /// Tries to deduct resources. Returns GameError::NotEnoughResources if funds are insufficient.
    pub fn deduct_resources(&mut self, cost: &ResourceGroup) -> Result<(), GameError> {
        if !self.has_enough_resources(cost) {
            return Err(GameError::NotEnoughResources);
        }
        self.stocks.remove_resources(cost);
        self.update_state();
        Ok(())
    }

    /// Stores resources in the village, respecting capacity.
    pub fn store_resources(&mut self, resources: &ResourceGroup) {
        self.stocks.store(resources);
        self.update_state();
    }

    /// Returns a snapshot of the currently stored resources.
    pub fn stored_resources(&self) -> ResourceGroup {
        self.stocks.stored()
    }

    /// Gets the current warehouse capacity.
    pub fn warehouse_capacity(&self) -> u32 {
        self.stocks.warehouse_capacity
    }

    /// Gets the current granary capacity.
    pub fn granary_capacity(&self) -> u32 {
        self.stocks.granary_capacity
    }

    /// Builds a new building on a given slot.
    pub fn add_building_at_slot(
        &mut self,
        building: Building,
        slot_id: u8,
    ) -> Result<(), GameError> {
        self.buildings.push(VillageBuilding { slot_id, building });
        self.update_state();

        Ok(())
    }

    /// Assigns a new level to a building in the given slot.
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
            .ok_or(GameError::EmptySlot { slot_id })?;

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
        if (1..=18).contains(&slot_id) {
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

    /// Returns all resource field buildings (slots 1-18) sorted by slot id.
    pub fn resource_fields(&self) -> Vec<VillageBuilding> {
        let mut fields = self
            .buildings
            .iter()
            .filter(|vb| (1..=RESOURCE_FIELDS_LAST_SLOT).contains(&vb.slot_id))
            .cloned()
            .collect::<Vec<_>>();
        fields.sort_by_key(|vb| vb.slot_id);
        fields
    }

    /// Returns a building in the village. Returns None if not present.
    /// In case of multiple buildings of same type, it returns the highest level one.
    pub fn get_building_by_name(&self, name: &BuildingName) -> Option<VillageBuilding> {
        if let Some(village_building) = self
            .buildings
            .iter()
            .filter(|&x| x.building.name == *name)
            .cloned()
            .max_by(|x, y| x.building.level.cmp(&y.building.level))
        {
            return Some(village_building);
        }
        None
    }

    /// Returns all building candidates that are valid for the given slot, ignoring requirements.
    pub fn candidate_buildings_for_slot(&self, slot_id: u8) -> Vec<BuildingName> {
        if slot_id == 0
            || slot_id > MAX_VILLAGE_SLOT_ID
            || slot_id <= RESOURCE_FIELDS_LAST_SLOT
            || self.get_building_by_slot_id(slot_id).is_some()
        {
            return vec![];
        }

        match slot_id {
            MAIN_BUILDING_SLOT_ID => vec![BuildingName::MainBuilding],
            RALLY_POINT_SLOT_ID => vec![BuildingName::RallyPoint],
            WALL_SLOT_ID => self.tribe.wall().into_iter().collect::<Vec<BuildingName>>(),
            _ => COMMON_BUILDINGS.to_vec(),
        }
    }

    /// Returns the list of buildings that can be constructed inside the given slot.
    /// Resource and reserved slots (main building, rally point, wall) always return empty lists.
    pub fn available_buildings_for_slot(&self, slot_id: u8) -> Vec<BuildingName> {
        self.candidate_buildings_for_slot(slot_id)
            .into_iter()
            .filter(|name| self.can_start_building(name))
            .collect()
    }

    fn can_start_building(&self, name: &BuildingName) -> bool {
        let building = Building::new(name.clone(), 1);
        self.validate_building_construction(&building).is_ok()
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
        if let Some(palace) = self.get_building_by_name(&BuildingName::Palace) {
            return Some((palace.building, BuildingName::Palace));
        }
        if let Some(residence) = self.get_building_by_name(&BuildingName::Residence) {
            return Some((residence.building, BuildingName::Residence));
        }
        None
    }

    /// Returns the village wall, if any, according to the actual tribe.
    pub fn wall(&self) -> Option<Building> {
        self.tribe
            .wall()
            .and_then(|building_name| self.get_building_by_name(&building_name))
            .map(|vb| vb.building)
    }

    /// Get defense bonus from wall, if any.
    pub fn wall_defense_bonus(&self) -> f64 {
        if let Some(wall) = self.wall() {
            return self.tribe.get_wall_factor().powf(wall.level as f64);
        }
        1.0
    }

    /// Returns MainBuilding level
    pub fn main_building_level(&self) -> u8 {
        self.get_building_by_name(&BuildingName::MainBuilding)
            .map_or(0, |vb| vb.building.level)
    }

    /// Get buildings durability, considering the level of StonemansionLodge, if any.
    pub fn buildings_durability(&self) -> f64 {
        match self.get_building_by_name(&BuildingName::StonemansionLodge) {
            Some(b) => 1.0 + b.building.level as f64 * 0.1,
            None => 1.0,
        }
    }

    pub fn loyalty(&self) -> u8 {
        self.loyalty
    }

    pub fn army(&self) -> Option<&Army> {
        self.army.as_ref()
    }

    pub fn set_army(&mut self, army: Option<&Army>) -> Result<(), GameError> {
        self.army = army.cloned();
        self.update_state();
        Ok(())
    }

    pub fn merge_army(&mut self, army: &Army) -> Result<(), GameError> {
        let mut home_army = self
            .army()
            .map_or(Army::new_village_army(self), |a| a.clone());
        home_army.merge(army)?;
        self.set_army(Some(&home_army))?;
        self.update_state();
        Ok(())
    }

    pub fn reinforcements(&self) -> &Vec<Army> {
        &self.reinforcements
    }

    pub fn add_reinforcements(&mut self, army: &Army) -> Result<(), GameError> {
        self.reinforcements.append(&mut vec![army.clone()]);
        Ok(())
    }

    pub fn deployed_armies(&self) -> &Vec<Army> {
        &self.deployed_armies
    }

    /// Applies losses and damages from the battle report to the village.
    pub fn apply_battle_report(
        &mut self,
        report: &BattleReport,
        server_speed: i8,
    ) -> Result<(), GameError> {
        if let Some(defender_report) = &report.defender
            && let Some(mut home_army) = self.army.take()
        {
            home_army.apply_battle_report(&defender_report);
            self.army = Some(home_army);
        }

        for report in &report.reinforcements {
            if let Some(index) = self
                .reinforcements
                .iter()
                .position(|r| r.id == report.army_before.id)
            {
                let army = &mut self.reinforcements[index].clone();
                army.apply_battle_report(&report);
                let _ = std::mem::replace(&mut self.reinforcements[index], army.clone());
            }
        }
        self.update_state();

        // Building damages

        // Wall damage
        if let Some(wall_damage) = &report.wall_damage
            && wall_damage.level_after < wall_damage.level_before
            && self.wall().is_some()
        {
            // FIXME: assume wall slot_id is always 19
            self.set_building_level_at_slot(19, wall_damage.level_after, server_speed)?;
        }

        // Catapult damages to other buildings
        for damage_report in &report.catapult_damage {
            if damage_report.level_after < damage_report.level_before
                && let Some(target) = self.get_building_by_name(&damage_report.name)
            {
                self.set_building_level_at_slot(
                    target.slot_id,
                    damage_report.level_after,
                    server_speed,
                )?;

                if damage_report.level_after == 0
                    && target.building.group != BuildingGroup::Resources
                {
                    self.remove_building_at_slot(target.slot_id, server_speed)?;
                }
            }
        }
        // Loyalty
        self.loyalty = report.loyalty_after;

        // Bounty
        if let Some(bounty) = &report.bounty {
            self.stocks.remove_resources(bounty);
        }

        self.update_state();
        Ok(())
    }

    /// Returns units available of a given group for training.
    pub fn available_units_for_training(&self, group: UnitGroup) -> Vec<&Unit> {
        self.tribe
            .units()
            .iter()
            .enumerate()
            .filter(|(idx, u)| self.academy_research().get(*idx) && u.group == group)
            .map(|(_, u)| u)
            .collect()
    }

    /// Returns available merchants.
    pub fn available_merchants(&self) -> u8 {
        self.total_merchants.saturating_sub(self.busy_merchants)
    }

    /// Marks a unit name as researched in the academy.
    pub fn research_academy(&mut self, unit: UnitName) -> Result<(), GameError> {
        if let Some(idx) = self.tribe.get_unit_idx_by_name(&unit) {
            self.academy_research.set(idx, true);
        }

        Ok(())
    }

    /// Upgrades smithy level for unit name.
    pub fn upgrade_smithy(&mut self, unit: UnitName) -> Result<(), GameError> {
        if let Some(idx) = self.tribe.get_unit_idx_by_name(&unit)
            && self.smithy[idx] < 20
        {
            self.smithy[idx] += 1;
        }

        Ok(())
    }

    #[cfg(any(test, feature = "test-utils"))]
    /// **[TEST ONLY]** Set academy research for specific unit.
    pub fn set_academy_research_for_test(&mut self, unit: &UnitName, is_researched: bool) {
        if let Some(idx) = self.tribe.get_unit_idx_by_name(unit) {
            self.academy_research.set(idx, is_researched);
        }
    }

    #[cfg(any(test, feature = "test-utils"))]
    /// **[TEST ONLY]** Set smithy level for specific unit.
    pub fn set_smithy_level_for_test(&mut self, unit: &UnitName, level: u8) {
        if let Some(idx) = self.tribe.get_unit_idx_by_name(unit) {
            self.smithy[idx] = level.min(20);
        }
    }

    /// Validates over a list of building requirements.
    pub fn validate_building_requirements(
        &self,
        requirements: &'static [BuildingRequirement],
    ) -> Result<(), GameError> {
        for req in requirements {
            if !self
                .buildings
                .iter()
                .any(|vb| vb.building.name == req.0 && vb.building.level >= req.1)
            {
                return Err(GameError::BuildingRequirementsNotMet {
                    building: req.0.clone(),
                    level: req.1,
                });
            }
        }

        Ok(())
    }

    /// Applies validations to build a construction.
    pub fn validate_building_construction(&self, building: &Building) -> Result<(), GameError> {
        let data = get_building_data(&building.name)?;

        // tribe constraints
        if !data.rules.tribes.is_empty() {
            let ok = data.rules.tribes.contains(&self.tribe);
            if !ok {
                return Err(GameError::BuildingTribeMismatch {
                    building: building.name.clone(),
                    tribe: self.tribe.clone(),
                });
            }
        }

        // capital/non-capital constraints
        if self.is_capital
            && data
                .rules
                .constraints
                .contains(&BuildingConstraint::NonCapital)
        {
            return Err(GameError::NonCapitalConstraint(building.name.clone()));
        }

        if !self.is_capital
            && data
                .rules
                .constraints
                .contains(&BuildingConstraint::OnlyCapital)
        {
            return Err(GameError::CapitalConstraint(building.name.clone()));
        }

        self.validate_building_requirements(data.rules.requirements)?;

        for vb in self.buildings.iter() {
            // check if a building has conflicts with other buildings (eg: Palace vs Residence)
            for conflict in data.rules.conflicts {
                if vb.building.name == conflict.0 {
                    return Err(GameError::BuildingConflict(
                        building.name.clone(),
                        conflict.0.clone(),
                    ));
                }
            }

            // rules for duplicated buildings (eg: Warehouse or Granary)
            if building.name == vb.building.name {
                // and allows multiple
                if !data.rules.allow_multiple {
                    return Err(GameError::NoMultipleBuildingConstraint(
                        building.name.clone(),
                    ));
                }
                // Require at least one existing building at max level before adding another copy
                let has_max_level_instance = self
                    .buildings
                    .iter()
                    .filter(|existing| existing.building.name == building.name)
                    .any(|existing| existing.building.level == data.rules.max_level);
                if !has_max_level_instance {
                    return Err(GameError::MultipleBuildingMaxNotReached(
                        building.name.clone(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Updates the village state (production, upkeep, etc...).
    fn update_state(&mut self) {
        self.population = 0;
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

    /// Sets total merchants count based on Marketplace level.
    fn update_merchants_count(&mut self) {
        if let Some(marketplace) = self.get_building_by_name(&BuildingName::Marketplace) {
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
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
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
        let lumber =
            ((self.lumber as f64) * ((self.bonus.lumber as f64 / 100.0) + 1.0)).floor() as u32;
        let clay = (self.clay as f64 * ((self.bonus.clay as f64 / 100.0) + 1.0)).floor() as u32;
        let iron = (self.iron as f64 * ((self.bonus.iron as f64 / 100.0) + 1.0)).floor() as u32;
        let mut crop = (self.crop as f64 * ((self.bonus.crop as f64 / 100.0) + 1.0)).floor() as i64;
        crop -= self.upkeep as i64;

        self.effective = VillageEffectiveProduction {
            lumber,
            clay,
            iron,
            crop,
        };
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

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct VillageEffectiveProduction {
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: i64,
}

// Bonus to be applied to resources production.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
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
    /// Returns the currently stored resources as ResourceGroup
    pub(crate) fn stored(&self) -> ResourceGroup {
        ResourceGroup::new(self.lumber, self.clay, self.iron, self.crop.max(0) as u32)
    }

    /// Stores resources into the village stocks, capping at storage capacity.
    pub(crate) fn store(&mut self, resources: &ResourceGroup) {
        self.lumber = (self.lumber + resources.lumber()).min(self.warehouse_capacity);
        self.clay = (self.clay + resources.clay()).min(self.warehouse_capacity);
        self.iron = (self.iron + resources.iron()).min(self.warehouse_capacity);
        self.crop = (self.crop + resources.crop() as i64).min(self.granary_capacity as i64);
    }

    /// Checks if given resources are present in stocks.
    pub(crate) fn has_availability(&self, resources: &ResourceGroup) -> bool {
        self.lumber >= resources.lumber()
            && self.clay >= resources.clay()
            && self.iron >= resources.iron()
            && self.crop >= resources.crop() as i64
    }

    /// Removes resources from the village stocks, ensuring they don't go negative.
    pub(crate) fn remove_resources(&mut self, resources: &ResourceGroup) {
        self.lumber = (self.lumber as i64 - resources.lumber() as i64).max(0) as u32;
        self.clay = (self.clay as i64 - resources.clay() as i64).max(0) as u32;
        self.iron = (self.iron as i64 - resources.iron() as i64).max(0) as u32;
        self.crop -= resources.crop() as i64; // crop can go negative
    }
}

impl Default for VillageStocks {
    fn default() -> Self {
        Self {
            warehouse_capacity: 800, // Base capacity
            granary_capacity: 800,   // Base capacity
            lumber: 800,
            clay: 800,
            iron: 800,
            crop: 800,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unnecessary_cast)]
mod tests {
    use crate::{
        models::{buildings::Building, village::VillageStocks},
        test_utils::{
            PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions, player_factory,
            valley_factory, village_factory,
        },
    };
    use chrono::{Duration, Utc};
    use parabellum_types::{
        buildings::BuildingName, errors::GameError, map::ValleyTopology, tribe::Tribe,
    };

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
        assert_eq!(v.production.upkeep, 2, "upkeep");

        // Population: 0 (base) + 2 (main building) = 2
        assert_eq!(v.population, 2, "population");

        // Stocks
        assert_eq!(v.stocks.warehouse_capacity, 800, "stock warehouse");
        assert_eq!(v.stocks.granary_capacity, 800, "stock granary");

        // Effective production
        // 12 crop - 2 upkeep = 10 effective crop
        assert_eq!(v.production.effective.crop, 10, "effective crop production");
    }

    #[test]
    fn test_multiple_buildings_require_max_level() {
        let mut v = village_factory(Default::default());
        let warehouse = Building::new(BuildingName::Warehouse, 1)
            .at_level(10, 1)
            .unwrap();
        v.add_building_at_slot(warehouse, 20).unwrap();

        let result = v.init_building_construction(21, BuildingName::Warehouse, 1);
        assert!(matches!(
            result,
            Err(GameError::MultipleBuildingMaxNotReached(
                BuildingName::Warehouse
            ))
        ));
    }

    #[test]
    fn test_multiple_buildings_allowed_after_max_level() {
        let mut v = village_factory(Default::default());
        let warehouse = Building::new(BuildingName::Warehouse, 1)
            .at_level(20, 1)
            .unwrap();
        v.add_building_at_slot(warehouse, 20).unwrap();

        let available = v.available_buildings_for_slot(21);
        assert!(available.contains(&BuildingName::Warehouse));
    }

    #[test]
    fn test_available_buildings_for_common_slot() {
        let v = village_factory(Default::default());
        let available = v.available_buildings_for_slot(20);

        assert!(
            available.contains(&BuildingName::Warehouse),
            "Warehouse should be available in empty slots"
        );
        assert!(
            available.contains(&BuildingName::Granary),
            "Granary should be available in empty slots"
        );
        assert!(
            available.contains(&BuildingName::Cranny),
            "Cranny should be available in empty slots"
        );
    }

    #[test]
    fn test_available_buildings_for_wall_slot_depends_on_tribe() {
        let roman_village = village_factory(Default::default());
        let roman_wall = roman_village.available_buildings_for_slot(40);
        assert_eq!(
            roman_wall,
            vec![BuildingName::CityWall],
            "Romans should only see City Wall in wall slot"
        );

        let teuton_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Teuton),
            ..Default::default()
        });
        let teuton_village = village_factory(VillageFactoryOptions {
            player: Some(teuton_player),
            ..Default::default()
        });
        let teuton_wall = teuton_village.available_buildings_for_slot(40);
        assert_eq!(
            teuton_wall,
            vec![BuildingName::EarthWall],
            "Teutons should only see Earth Wall in wall slot"
        );
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

        v.update_state();

        let expected_resources = (effective_prod_per_sec as f64 * 3600.0).floor() as u32;

        assert_eq!(expected_resources, 8);
        assert_eq!(v.stocks.lumber, 8, "Lumber should be 8 after 1 hour");
        assert_eq!(v.stocks.clay, 8, "Clay should be 8 after 1 hour");
        assert_eq!(v.stocks.iron, 8, "Iron should be 8 after 1 hour");
        assert_eq!(v.stocks.crop, 10, "Crop should be 10 after 1 hour");

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
        assert_eq!(v.population, 2, "Initial village population should be 4");
        assert_eq!(v.production.upkeep, 2, "Initial village upkeep should be 4");

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
            v.population, 3,
            "Village population with MainBuilding L2 should be 3"
        );
        assert_eq!(
            v.production.upkeep, 3,
            "Village upkeep with MainBuilding L2 should be 3"
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
            v.population, 4,
            "Village population with Main Building L3 should be 4"
        );
        assert_eq!(
            v.production.upkeep, 4,
            "Village upkeep with Main Building L3 upgrade should be 4"
        );
    }
}
