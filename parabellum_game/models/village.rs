use chrono::{DateTime, Utc};
use parabellum_types::army::{Unit, UnitGroup, UnitRole};
use rand::seq::IndexedRandom;
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
    pub culture_points: u32,
    pub culture_points_production: u32,
    pub updated_at: DateTime<Utc>,
    pub parent_village_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageSnapshot {
    pub id: u32,
    pub name: String,
    pub player_id: Uuid,
    pub position: Position,
    pub tribe: Tribe,
    pub buildings: Vec<VillageBuilding>,
    pub oases: Vec<Oasis>,
    pub army: Option<Army>,
    pub reinforcements: Vec<Army>,
    pub deployed_armies: Vec<Army>,
    pub loyalty: u8,
    pub is_capital: bool,
    pub smithy: SmithyUpgrades,
    pub stocks: VillageStocks,
    pub academy_research: AcademyResearch,
    pub culture_points: u32,
    pub updated_at: DateTime<Utc>,
    pub parent_village_id: Option<u32>,
}

impl Village {
    fn horse_drinking_trough_level(&self) -> u8 {
        self.get_building_by_name(&BuildingName::HorseDrinkingTrough)
            .map(|b| b.building.level)
            .unwrap_or(0)
    }

    pub fn cavalry_training_time_multiplier(&self, unit: &Unit) -> f64 {
        if self.tribe != Tribe::Roman || unit.group != UnitGroup::Cavalry {
            return 1.0;
        }
        let trough_level = self.horse_drinking_trough_level() as f64;
        (100.0 - trough_level).max(1.0) / 100.0
    }

    fn unit_upkeep_with_trough(unit: &Unit, trough_level: u8) -> u32 {
        let discount = match unit.name {
            UnitName::EquitesLegati if trough_level >= 10 => 1,
            UnitName::EquitesImperatoris if trough_level >= 15 => 1,
            UnitName::EquitesCaesaris if trough_level >= 20 => 1,
            _ => 0,
        };
        unit.cost.upkeep.saturating_sub(discount)
    }

    pub fn effective_unit_upkeep(&self, unit: &Unit) -> u32 {
        let trough_level = self.horse_drinking_trough_level();
        Self::unit_upkeep_with_trough(unit, trough_level)
    }

    fn army_upkeep_with_trough(&self, army: &Army) -> u32 {
        let trough_level = self.horse_drinking_trough_level();
        let units_data = army.tribe.units();
        let mut total = 0u32;
        for (idx, qty) in army.units().units().iter().enumerate() {
            let Some(unit) = units_data.get(idx) else {
                continue;
            };
            total = total.saturating_add(Self::unit_upkeep_with_trough(unit, trough_level) * qty);
        }
        total
    }

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
            culture_points: 0,
            culture_points_production: 0,
            stocks: VillageStocks::default_for_speed(server_speed),
            updated_at: Utc::now(),
            parent_village_id: None,
        };

        // FIXME: either fix the method return value or this method one.
        village
            .init_village_buildings(valley, server_speed)
            .unwrap();
        village.update_state();
        village
    }

    /// Rehydrates a Village domain model from a domain snapshot.
    pub fn rehydrate(snapshot: VillageSnapshot) -> Self {
        let mut village = Self {
            id: snapshot.id,
            name: snapshot.name,
            player_id: snapshot.player_id,
            position: snapshot.position,
            tribe: snapshot.tribe,
            buildings: snapshot.buildings,
            oases: snapshot.oases,
            population: 0,
            army: snapshot.army,
            reinforcements: snapshot.reinforcements,
            deployed_armies: snapshot.deployed_armies,
            loyalty: snapshot.loyalty,
            production: VillageProduction::default(),
            is_capital: snapshot.is_capital,
            smithy: snapshot.smithy,
            stocks: snapshot.stocks,
            academy_research: snapshot.academy_research,
            total_merchants: 0,
            busy_merchants: 0,
            culture_points: snapshot.culture_points,
            culture_points_production: 0,
            updated_at: snapshot.updated_at,
            parent_village_id: snapshot.parent_village_id,
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
        let trough_multiplier = self.cavalry_training_time_multiplier(&unit);

        let time_per_unit = (base_time_per_unit * training_time_bonus_perc * trough_multiplier)
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

    /// Prepares for upgrading a building, applies validations and withdraws resources.
    /// Returns the upgraded building name, target level, and construction time.
    pub fn init_building_upgrade(
        &mut self,
        slot_id: u8,
        server_speed: i8,
    ) -> Result<(BuildingName, u8, u32), GameError> {
        let current = self
            .get_building_by_slot_id(slot_id)
            .ok_or(GameError::EmptySlot { slot_id })?;
        let data = get_building_data(&current.building.name)?;

        if current.building.level >= data.rules.max_level {
            return Err(GameError::BuildingMaxLevelReached);
        }

        let next_level = current.building.level + 1;
        let target = Building::new(current.building.name.clone(), server_speed)
            .at_level(next_level, server_speed)?;

        self.validate_building_requirements(data.rules.requirements)?;
        self.deduct_resources(&target.cost().resources)?;

        let mb_level = self.main_building_level();
        Ok((
            current.building.name,
            next_level,
            target.calculate_build_time_secs(&server_speed, &mb_level),
        ))
    }

    /// Prepares for downgrading a building. Returns the building name, target level,
    /// and construction time.
    pub fn init_building_downgrade(
        &self,
        slot_id: u8,
        server_speed: i8,
    ) -> Result<(BuildingName, u8, u32), GameError> {
        if self.main_building_level() < 10 {
            return Err(GameError::BuildingRequirementsNotMet {
                building: BuildingName::MainBuilding,
                level: 10,
            });
        }

        let current = self
            .get_building_by_slot_id(slot_id)
            .ok_or(GameError::EmptySlot { slot_id })?;
        if current.building.level == 0 {
            return Err(GameError::InvalidBuildingLevel(
                0,
                current.building.name.clone(),
            ));
        }

        let next_level = current.building.level - 1;
        let target = Building::new(current.building.name.clone(), server_speed)
            .at_level(next_level, server_speed)?;
        let mb_level = self.main_building_level();

        Ok((
            current.building.name,
            next_level,
            target.calculate_build_time_secs(&server_speed, &mb_level),
        ))
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

        if unit.research_cost.time > 0 && !self.academy_research().get(unit_idx) {
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
        self.update_state();
        if !self.has_enough_resources(cost) {
            return Err(GameError::NotEnoughResources);
        }
        self.stocks.remove_resources(cost);
        Ok(())
    }

    /// Stores resources in the village, respecting capacity.
    pub fn store_resources(&mut self, resources: &ResourceGroup) {
        self.update_state();
        self.stocks.store(resources);
    }

    /// Reserves resources and merchants for an outgoing merchant workflow.
    pub fn reserve_merchant_transfer(
        &mut self,
        resources: &ResourceGroup,
        merchants_used: u8,
    ) -> Result<(), GameError> {
        self.deduct_resources(resources)?;
        self.busy_merchants = self.busy_merchants.saturating_add(merchants_used);
        Ok(())
    }

    /// Releases resources and merchants from a reserved merchant workflow.
    pub fn release_merchant_transfer(&mut self, resources: &ResourceGroup, merchants_used: u8) {
        self.store_resources(resources);
        self.busy_merchants = self.busy_merchants.saturating_sub(merchants_used);
    }

    /// Marks merchants as returned after an outgoing merchant workflow completes.
    pub fn return_merchants(&mut self, merchants_used: u8) {
        self.busy_merchants = self.busy_merchants.saturating_sub(merchants_used);
    }

    /// Returns a snapshot of the currently stored resources.
    pub fn stored_resources(&self) -> ResourceGroup {
        self.stocks.stored()
    }

    /// Standard resource cost for founding a new village with settlers.
    pub fn foundation_cost() -> ResourceGroup {
        ResourceGroup::new(800, 800, 800, 800)
    }

    /// Withdraws the standard settler foundation resources.
    pub fn deduct_foundation_resources(&mut self) -> Result<(), GameError> {
        self.deduct_resources(&Self::foundation_cost())
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
        let mut rng = rand::rng();
        let buildings: Vec<Building> = self
            .buildings
            .iter()
            .map(|vb| vb.building.clone())
            .collect();

        buildings
            .sample(&mut rng, count)
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

    /// Returns the maximum number of foundation slots available based on Palace/Residence level.
    /// Residence: Level 10 → 1 slot, Level 20 → 2 slots
    /// Palace: Level 10 → 1 slot, Level 15 → 2 slots, Level 20 → 3 slots
    pub fn max_foundation_slots(&self) -> u8 {
        if let Some((building, building_name)) = self.get_palace_or_residence() {
            match building_name {
                BuildingName::Residence => {
                    if building.level >= 20 {
                        2
                    } else if building.level >= 10 {
                        1
                    } else {
                        0
                    }
                }
                BuildingName::Palace => {
                    if building.level >= 20 {
                        3
                    } else if building.level >= 15 {
                        2
                    } else if building.level >= 10 {
                        1
                    } else {
                        0
                    }
                }
                _ => 0,
            }
        } else {
            0
        }
    }

    /// Returns the number of settlers currently in the home army (not deployed).
    pub fn count_settlers_at_home(&self) -> u32 {
        if let Some(army) = &self.army {
            army.units().get(9)
        } else {
            0
        }
    }

    /// Returns the number of chiefs/senators/chieftains currently in the home army (not deployed).
    pub fn count_chiefs_at_home(&self) -> u32 {
        if let Some(army) = &self.army {
            army.units().get(8)
        } else {
            0
        }
    }

    /// Constants for expansion units
    const SETTLERS_PER_SLOT: u32 = 3;
    const CHIEFS_PER_SLOT: u32 = 1;

    /// Calculate slots used by settlers (rounds up: 1-3 settlers = 1 slot)
    pub fn slots_used_by_settlers(settlers_count: u32) -> u32 {
        if settlers_count == 0 {
            0
        } else {
            settlers_count.div_ceil(Self::SETTLERS_PER_SLOT)
        }
    }

    /// Calculate slots used by chiefs (1 chief = 1 slot)
    pub fn slots_used_by_chiefs(chiefs_count: u32) -> u32 {
        chiefs_count * Self::CHIEFS_PER_SLOT
    }

    /// Calculate maximum settlers trainable given available slots
    pub fn max_settlers_for_slots(available_slots: u8) -> u32 {
        available_slots as u32 * Self::SETTLERS_PER_SLOT
    }

    /// Calculate maximum chiefs trainable given available slots
    pub fn max_chiefs_for_slots(available_slots: u8) -> u32 {
        available_slots as u32 * Self::CHIEFS_PER_SLOT
    }

    /// Calculate max trainable quantity for an expansion unit
    ///
    /// Accounts for:
    /// - Available foundation slots
    /// - Slots used by the other expansion unit type
    /// - Units already committed (at home, deployed, in training)
    pub fn max_expansion_unit_trainable(
        unit_role: UnitRole,
        available_slots: u8,
        total_chiefs: u32,
        total_settlers: u32,
        committed_this_unit: u32,
    ) -> u32 {
        if available_slots == 0 {
            return 0;
        }

        let max_allowed = match unit_role {
            UnitRole::Chief => {
                // Chiefs: 1 per slot, minus slots used by settlers
                let slots_used_by_settlers = Self::slots_used_by_settlers(total_settlers);
                let slots_for_chiefs = available_slots.saturating_sub(slots_used_by_settlers as u8);
                Self::max_chiefs_for_slots(slots_for_chiefs)
            }
            UnitRole::Settler => {
                // Settlers: 3 per slot, minus slots used by chiefs
                let slots_used_by_chiefs = Self::slots_used_by_chiefs(total_chiefs);
                let slots_for_settlers = available_slots.saturating_sub(slots_used_by_chiefs as u8);
                Self::max_settlers_for_slots(slots_for_settlers)
            }
            _ => 0,
        };

        max_allowed.saturating_sub(committed_this_unit)
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

    /// Returns village loyalty.
    pub fn loyalty(&self) -> u8 {
        self.loyalty
    }

    pub fn regenerate_loyalty_to(&mut self, loyalty_after: u8) {
        self.loyalty = loyalty_after.min(100);
    }

    /// Calculates the total culture points production per day from all buildings.
    pub fn calculate_culture_points_production(&self) -> u32 {
        self.buildings
            .iter()
            .map(|vb| vb.building.culture_points as u32)
            .sum()
    }

    /// Returns home army, if any.
    pub fn army(&self) -> Option<&Army> {
        self.army.as_ref()
    }

    /// Set home army.
    pub fn set_army(&mut self, army: Option<&Army>) -> Result<(), GameError> {
        self.army = army.cloned();
        self.update_state();
        Ok(())
    }

    /// Merges a given army to the current home army.
    pub fn merge_army(&mut self, army: &Army) -> Result<(), GameError> {
        let mut home_army = self
            .army()
            .map_or(Army::new_village_army(self), |a| a.clone());
        home_army.merge(army)?;
        if home_army.hero().is_none() {
            home_army.set_hero(army.hero());
        }
        self.set_army(Some(&home_army))?;
        self.update_state();
        Ok(())
    }

    /// Adds freshly trained units to the home army.
    pub fn add_trained_units_home(
        &mut self,
        unit: UnitName,
        quantity: u32,
    ) -> Result<(), GameError> {
        let mut trained = Army::new_village_army(self);
        trained.add_unit(unit, quantity)?;
        self.merge_army(&trained)
    }

    /// Returns reinforcements in village.
    pub fn reinforcements(&self) -> &Vec<Army> {
        &self.reinforcements
    }

    /// Adds an army to the reinforcements in the village, merging with an existing
    /// reinforcement from the same sender (player + origin village) if present.
    pub fn add_reinforcements(&mut self, army: &Army) -> Result<(), GameError> {
        if let Some(idx) = self
            .reinforcements
            .iter()
            .position(|r| r.player_id == army.player_id && r.village_id == army.village_id)
        {
            let mut existing = self.reinforcements[idx].clone();
            existing.merge(army)?;
            existing.current_map_field_id = Some(self.id);
            self.reinforcements[idx] = existing;
            self.update_state();
            Ok(())
        } else {
            let mut incoming = army.clone();
            incoming.current_map_field_id = Some(self.id);
            self.reinforcements.push(incoming);
            self.update_state();
            Ok(())
        }
    }

    /// Returns the list of armies sent elsewhere.
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
            home_army.apply_battle_report(defender_report);
            self.army = if home_army.immensity() > 0 {
                Some(home_army)
            } else {
                None
            };
        }

        for report in &report.reinforcements {
            if let Some(index) = self
                .reinforcements
                .iter()
                .position(|r| r.id == report.army_before.id)
            {
                let army = &mut self.reinforcements[index].clone();
                army.apply_battle_report(report);
                let _ = std::mem::replace(&mut self.reinforcements[index], army.clone());
            }
        }
        self.reinforcements.retain(|army| army.immensity() > 0);
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
                && let Some(target) = self
                    .buildings
                    .iter()
                    .filter(|b| b.building.name == damage_report.name)
                    .find(|b| b.building.level == damage_report.level_before)
                    .or_else(|| {
                        self.buildings
                            .iter()
                            .filter(|b| b.building.name == damage_report.name)
                            .max_by_key(|b| b.building.level)
                    })
                    .cloned()
            {
                let next_level = damage_report.level_after.min(target.building.level);
                self.set_building_level_at_slot(target.slot_id, next_level, server_speed)?;

                if next_level == 0 && target.building.group != BuildingGroup::Resources {
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
            .filter(|(idx, u)| {
                (self.academy_research().get(*idx) || u.research_cost.time == 0) && u.group == group
            })
            .map(|(_, u)| u)
            .collect()
    }

    /// Returns available merchants.
    pub fn available_merchants(&self) -> u8 {
        self.total_merchants.saturating_sub(self.busy_merchants)
    }

    /// Returns merchant carrying capacity after applying server speed.
    pub fn merchant_capacity(&self, server_speed: i8) -> u32 {
        let speed_multiplier = server_speed.max(1) as u32;
        self.tribe
            .merchant_stats()
            .capacity
            .saturating_mul(speed_multiplier)
    }

    /// Returns merchants required to move the requested resources.
    pub fn required_merchants(
        &self,
        resources: &ResourceGroup,
        server_speed: i8,
    ) -> Result<u8, GameError> {
        let capacity = self.merchant_capacity(server_speed);
        if capacity == 0 {
            return Err(GameError::NotEnoughMerchants);
        }

        let total = resources.total();
        let needed = ((total as f64) / (capacity as f64)).ceil() as u8;
        let merchants_needed = if total > 0 { needed.max(1) } else { 0 };
        if merchants_needed == 0 || merchants_needed > self.available_merchants() {
            return Err(GameError::NotEnoughMerchants);
        }

        Ok(merchants_needed)
    }

    /// Validates and returns merchants required for a resource transfer.
    pub fn validate_merchant_transfer(
        &self,
        resources: &ResourceGroup,
        server_speed: i8,
    ) -> Result<u8, GameError> {
        if self
            .get_building_by_name(&BuildingName::Marketplace)
            .is_none_or(|slot| slot.building.level == 0)
        {
            return Err(GameError::BuildingRequirementsNotMet {
                building: BuildingName::Marketplace,
                level: 1,
            });
        }
        if !self.has_enough_resources(resources) {
            return Err(GameError::NotEnoughResources);
        }

        self.required_merchants(resources, server_speed)
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
        let default_capacity = 800 * self.inferred_server_speed().max(1) as u32;

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
        if self.stocks.granary_capacity == 0 {
            self.stocks.granary_capacity = default_capacity;
        }
        if self.stocks.warehouse_capacity == 0 {
            self.stocks.warehouse_capacity = default_capacity;
        }

        // population upkeep
        self.production.upkeep += self.population;

        // oases production bonuses
        for o in self.oases.clone() {
            let oasis_bonus = o.bonus();
            self.production.bonus.add(&oasis_bonus)
        }

        // armies upkeep
        self.production.upkeep += self
            .army
            .clone()
            .map_or(0, |a| self.army_upkeep_with_trough(&a));
        for a in self.reinforcements.iter() {
            self.production.upkeep += self.army_upkeep_with_trough(a);
        }

        // update internal data
        self.production.calculate_effective_production();
        self.update_merchants_count();
        self.update_culture_points_production();
        self.update_resources();
    }

    /// Updates culture points production based on all buildings.
    fn update_culture_points_production(&mut self) {
        self.culture_points_production = self.calculate_culture_points_production();
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
        if self.updated_at > now {
            // Guard against local clock skew/sleep/manual time adjustments:
            // keep state monotonic so resource growth can resume on next reads/actions.
            self.updated_at = now;
            return;
        }
        let time_elapsed = (now - self.updated_at).num_seconds() as f64;

        if time_elapsed <= 0.0 {
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

    fn inferred_server_speed(&self) -> i8 {
        self.buildings
            .iter()
            .filter_map(|building| building.building.inferred_server_speed())
            .max()
            .unwrap_or(1)
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
    pub fn default_for_speed(server_speed: i8) -> Self {
        let capacity = 800 * server_speed.max(1) as u32;
        Self {
            warehouse_capacity: capacity,
            granary_capacity: capacity,
            lumber: 800,
            clay: 800,
            iron: 800,
            crop: 800,
        }
    }

    /// Returns the currently stored resources as ResourceGroup
    pub fn stored(&self) -> ResourceGroup {
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
        models::{
            army::Army,
            buildings::Building,
            village::{VillageBuilding, VillageStocks},
        },
        test_utils::{
            PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions, player_factory,
            valley_factory, village_factory,
        },
    };
    use chrono::{Duration, Utc};
    use parabellum_types::{
        army::{TroopSet, UnitName},
        buildings::{BuildingGroup, BuildingName},
        common::ResourceGroup,
        errors::GameError,
        map::ValleyTopology,
        tribe::Tribe,
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
    fn initial_storage_capacity_scales_with_server_speed_without_storage_buildings() {
        let village = village_factory(VillageFactoryOptions {
            server_speed: Some(3),
            ..Default::default()
        });

        assert_eq!(village.stocks.warehouse_capacity, 2400);
        assert_eq!(village.stocks.granary_capacity, 2400);
    }

    #[test]
    fn merchant_transfer_reserve_and_release_updates_resources_and_busy_merchants() {
        let mut village = village_factory(Default::default());
        village.buildings.push(VillageBuilding {
            slot_id: 27,
            building: Building {
                name: BuildingName::Marketplace,
                group: BuildingGroup::Infrastructure,
                value: 0,
                population: 0,
                culture_points: 0,
                level: 2,
            },
        });
        village.update_state();
        village
            .deduct_resources(&ResourceGroup::new(300, 0, 0, 0))
            .unwrap();

        village
            .reserve_merchant_transfer(&ResourceGroup::new(200, 0, 0, 0), 1)
            .unwrap();

        assert_eq!(village.busy_merchants, 1);
        assert_eq!(village.stored_resources().lumber(), 300);

        village.release_merchant_transfer(&ResourceGroup::new(200, 0, 0, 0), 1);

        assert_eq!(village.busy_merchants, 0);
        assert_eq!(village.stored_resources().lumber(), 500);
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
    fn test_update_resources_x5_elapsed_growth_matches_effective_hourly_production() {
        let mut v = village_factory(VillageFactoryOptions {
            server_speed: Some(5),
            ..Default::default()
        });

        // Start from empty stocks and simulate exactly one hour elapsed.
        v.stocks = VillageStocks {
            lumber: 0,
            clay: 0,
            iron: 0,
            crop: 0,
            ..v.stocks
        };
        v.updated_at = Utc::now() - Duration::seconds(3600);

        let expected_lumber = v.production.effective.lumber;
        let expected_clay = v.production.effective.clay;
        let expected_iron = v.production.effective.iron;
        let expected_crop = v.production.effective.crop;

        v.update_state();

        assert_eq!(v.stocks.lumber, expected_lumber);
        assert_eq!(v.stocks.clay, expected_clay);
        assert_eq!(v.stocks.iron, expected_iron);
        assert_eq!(v.stocks.crop, expected_crop);
    }

    #[test]
    fn test_update_resources_clamps_future_updated_at_and_keeps_stocks_unchanged() {
        let mut v = village_factory(Default::default());
        let before_updated_at = Utc::now() + Duration::hours(2);
        v.updated_at = before_updated_at;
        let before_stocks = v.stocks.clone();

        v.update_state();

        assert!(v.updated_at <= Utc::now());
        assert_eq!(v.stocks.lumber, before_stocks.lumber);
        assert_eq!(v.stocks.clay, before_stocks.clay);
        assert_eq!(v.stocks.iron, before_stocks.iron);
        assert_eq!(v.stocks.crop, before_stocks.crop);
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

    #[test]
    fn test_deduct_resources_applies_elapsed_production_before_validation() {
        let mut v = village_factory(VillageFactoryOptions {
            ..Default::default()
        });

        // Start with empty stocks, then rewind time so production can accrue.
        v.stocks = VillageStocks {
            lumber: 0,
            clay: 0,
            iron: 0,
            crop: 0,
            ..v.stocks
        };
        v.updated_at = Utc::now() - Duration::hours(2);

        // Should pass because elapsed production is applied before availability check.
        let cost = ResourceGroup::new(1, 1, 1, 1);
        assert!(v.deduct_resources(&cost).is_ok());
    }

    #[test]
    fn test_horse_drinking_trough_reduces_cavalry_training_time() {
        let mut baseline = village_factory(Default::default());
        let stable = Building::new(BuildingName::Stable, 1)
            .at_level(1, 1)
            .unwrap();
        baseline.add_building_at_slot(stable.clone(), 21).unwrap();
        baseline.set_academy_research_for_test(&UnitName::EquitesLegati, true);
        let unit_idx = baseline
            .tribe
            .get_unit_idx_by_name(&UnitName::EquitesLegati)
            .unwrap() as u8;
        let (_, _, base_time_per_unit) = baseline
            .init_unit_training(unit_idx, &BuildingName::Stable, 1, 1)
            .unwrap();

        let mut v = village_factory(Default::default());
        let stable = Building::new(BuildingName::Stable, 1)
            .at_level(1, 1)
            .unwrap();
        v.add_building_at_slot(stable, 21).unwrap();
        let trough = Building::new(BuildingName::HorseDrinkingTrough, 1)
            .at_level(20, 1)
            .unwrap();
        v.add_building_at_slot(trough, 20).unwrap();
        v.set_academy_research_for_test(&UnitName::EquitesLegati, true);

        let (_, _, time_per_unit) = v
            .init_unit_training(unit_idx, &BuildingName::Stable, 1, 1)
            .unwrap();

        assert_eq!(
            time_per_unit,
            (base_time_per_unit as f64 * 0.8).floor() as u32
        );
    }

    #[test]
    fn test_horse_drinking_trough_reduces_roman_cavalry_upkeep_at_thresholds() {
        let mut v = village_factory(Default::default());
        let trough = Building::new(BuildingName::HorseDrinkingTrough, 1)
            .at_level(20, 1)
            .unwrap();
        v.add_building_at_slot(trough, 20).unwrap();

        let mut units = TroopSet::default();
        let legati_idx = v
            .tribe
            .get_unit_idx_by_name(&UnitName::EquitesLegati)
            .unwrap();
        let imperatoris_idx = v
            .tribe
            .get_unit_idx_by_name(&UnitName::EquitesImperatoris)
            .unwrap();
        let caesaris_idx = v
            .tribe
            .get_unit_idx_by_name(&UnitName::EquitesCaesaris)
            .unwrap();
        units.set(legati_idx, 1);
        units.set(imperatoris_idx, 1);
        units.set(caesaris_idx, 1);

        let army = Army::new(
            None,
            v.id,
            Some(v.id),
            v.player_id,
            v.tribe.clone(),
            &units,
            &[0; 8],
            None,
        );
        v.set_army(Some(&army)).unwrap();
        v.update_state();

        // Base upkeep for Roman cavalry trio: 2 + 3 + 4 = 9.
        // At trough level 20 each gets -1 => 6 total.
        assert_eq!(v.production.upkeep, v.population + 6);
    }
}
