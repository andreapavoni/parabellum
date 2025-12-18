use serde::{Deserialize, Serialize};

use crate::{
    buildings::{BuildingName, BuildingRequirement},
    common::{Cost, ResearchCost},
};

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TroopSet {
    units: [u32; 10],
}

impl TroopSet {
    /// Inits a new TroopSet.
    pub fn new(units: [u32; 10]) -> Self {
        Self { units }
    }

    /// Returns the quantity of units at given index.
    pub fn get(&self, idx: usize) -> u32 {
        self.units[idx]
    }

    /// Sets the quantity of units at given index.
    pub fn set(&mut self, idx: usize, quantity: u32) {
        self.units[idx] = quantity;
    }

    /// Add the quantity of units at given index.
    pub fn add(&mut self, idx: usize, quantity: u32) {
        self.units[idx] += quantity;
    }

    /// Removes the quantity of units at given index.
    pub fn remove(&mut self, idx: usize, quantity: u32) {
        self.units[idx] -= quantity;
    }

    /// Returns the units in the TroopSet.
    pub fn units(&self) -> &[u32; 10] {
        &self.units
    }

    /// Returns immensity of the TroopSet, but doesn't include hero.
    pub fn immensity(&self) -> u32 {
        self.units.iter().sum::<u32>()
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
    pub research_cost: ResearchCost,
    pub requirements: &'static [BuildingRequirement],
    pub buildings: &'static [BuildingName],
}

impl Unit {
    pub fn apply_smithy_upgrade(&self, smithy_level: i32, upkeep: u32, combat_value: u32) -> u32 {
        ((combat_value as f64)
            + ((combat_value + 300 * upkeep) as f64 / 7.0)
                * ((1.007f64).powi(smithy_level) - 1.0).floor()) as u32
    }

    pub fn get_requirements(&self) -> &'static [BuildingRequirement] {
        self.requirements
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnitGroup {
    Infantry,
    Cavalry,
    Siege,
    Expansion,
}

#[derive(Debug, Clone)]
pub struct UnitRequirement {
    pub building: BuildingName,
    pub level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
