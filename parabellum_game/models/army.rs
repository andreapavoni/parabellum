use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_types::{
    army::{Unit, UnitName, UnitRole},
    errors::GameError,
    tribe::Tribe,
};

use crate::{
    battle::BattlePartyReport,
    models::{hero::Hero, village::Village},
};

use super::smithy::SmithyUpgrades;

pub type TroopSet = [u32; 10];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Army {
    pub id: Uuid,
    pub village_id: u32,
    pub current_map_field_id: Option<u32>, // this army could have been deployed in some other Village or Oasis
    pub player_id: Uuid,
    pub tribe: Tribe,
    units: TroopSet,
    smithy: SmithyUpgrades,
    hero: Option<Hero>,
}

impl Army {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Option<Uuid>,
        village_id: u32,
        current_map_field_id: Option<u32>,
        player_id: Uuid,
        tribe: Tribe,
        units: &TroopSet,
        smithy: &SmithyUpgrades,
        hero: Option<Hero>,
    ) -> Self {
        Army {
            id: id.unwrap_or(Uuid::new_v4()),
            village_id,
            player_id,
            tribe,
            units: *units,
            smithy: *smithy,
            hero,
            current_map_field_id,
        }
    }

    /// Helper to get a new empty army for a specific village.
    pub fn new_village_army(village: &Village) -> Self {
        Army::new(
            None,
            village.id,
            Some(village.id),
            village.player_id,
            village.tribe.clone(),
            &[0; 10],
            village.smithy(),
            None,
        )
    }

    pub fn hero(&self) -> Option<Hero> {
        self.hero.clone()
    }

    pub fn set_hero(&mut self, hero: Option<Hero>) {
        self.hero = hero.clone();
    }

    pub fn smithy(&self) -> &SmithyUpgrades {
        &self.smithy
    }

    pub fn units(&self) -> &TroopSet {
        &self.units
    }

    /// Returns the amount of a given unit.
    pub fn unit_amount(&self, idx: u8) -> u32 {
        self.units[idx as usize]
    }

    /// Returns the total raw number of troops in the army.
    pub fn immensity(&self) -> u32 {
        let hero_count: u32 = self.hero.as_ref().map(|_| 1).unwrap_or(0);
        self.units.iter().sum::<u32>() + hero_count
    }

    /// Update units and hero in the army.
    pub fn apply_battle_report(&mut self, report: &BattlePartyReport) {
        self.update_units(&report.survivors);
        if let Some(mut hero) = self.hero() {
            hero.apply_battle_damage(report.loss_percentage);
            hero.gain_experience(report.hero_exp_gained);
            self.hero = Some(hero);
        }
    }

    /// Returns the total upkeep cost of the army.
    pub fn upkeep(&self) -> u32 {
        let units = self.tribe.units();
        let mut total: u32 = 0;

        for (idx, quantity) in self.units.iter().enumerate() {
            total += units[idx].cost.upkeep * quantity;
        }

        total
    }

    /// Returns the total capacity of the army.
    pub fn bounty_capacity(&self) -> u32 {
        self.bounty_capacity_troop_set(&self.units)
    }

    /// Returns the total capacity of a given troop set.
    pub fn bounty_capacity_troop_set(&self, troops: &TroopSet) -> u32 {
        let mut capacity: u32 = 0;
        let units_data = self.tribe.units();

        for (idx, &quantity) in troops.iter().enumerate() {
            if quantity > 0 {
                capacity += units_data[idx].capacity * quantity;
            }
        }

        capacity
    }

    /// Returns the total attack points of the army, split between infantry and cavalry.
    pub fn attack_points(&self) -> (u32, u32) {
        let mut infantry_points: u32 = 0;
        let mut cavalry_points: u32 = 0;

        for (idx, quantity) in self.units.iter().enumerate() {
            let u = self.get_unit_by_idx(idx as u8).unwrap();
            match u.role {
                UnitRole::Settler | UnitRole::Chief => continue,
                _ => (),
            }

            let smithy_improvement = self.apply_smithy_upgrade(&u, idx, u.attack);

            match u.role {
                UnitRole::Cavalry => cavalry_points += smithy_improvement * quantity,
                _ => infantry_points += smithy_improvement * quantity,
            }
        }
        (infantry_points, cavalry_points)
    }

    /// Returns the total attack points of the army, split between infantry and cavalry.
    pub fn defense_points(&self) -> (u32, u32) {
        let mut infantry_points: u32 = 0;
        let mut cavalry_points: u32 = 0;

        for (idx, quantity) in self.units.into_iter().enumerate() {
            let u = self.get_unit_by_idx(idx as u8).unwrap();

            match u.role {
                UnitRole::Settler | UnitRole::Chief => continue,
                _ => (),
            }

            let smithy_infantry = self.apply_smithy_upgrade(&u, idx, u.defense_infantry);
            let smithy_cavalry = self.apply_smithy_upgrade(&u, idx, u.defense_cavalry);

            infantry_points += smithy_infantry * quantity;
            cavalry_points += smithy_cavalry * quantity;
        }
        (infantry_points, cavalry_points)
    }

    /// Returns the scouting attack points of the army.
    pub fn scouting_attack_points(&self) -> u32 {
        self.scouting_points(35)
    }

    /// Returns the scouting defense points of the army.
    pub fn scouting_defense_points(&self) -> u32 {
        self.scouting_points(20)
    }

    /// Updates the current army and returns new deployed army.
    pub fn deploy(&mut self, set: TroopSet, hero: Option<Hero>) -> Result<Self, GameError> {
        for (idx, quantity) in set.iter().enumerate() {
            if *quantity == 0 {
                continue;
            }

            if self.units[idx] >= *quantity {
                self.units[idx] -= *quantity;
            } else {
                return Err(GameError::NotEnoughUnits);
            }
        }

        if hero.is_some() {
            self.hero = None;
        }

        let deployed = Self::new(
            None,
            self.village_id,
            None,
            self.player_id,
            self.tribe.clone(),
            &set,
            &self.smithy,
            hero,
        );

        Ok(deployed)
    }

    /// Returns the actual speed of the Army by taking the speed of slowest unit.
    pub fn speed(&self) -> u8 {
        let mut speed: u8 = 0;
        for (idx, quantity) in self.units.into_iter().enumerate() {
            if quantity > 0 {
                let u = self.get_unit_by_idx(idx as u8).unwrap();
                if speed == 0 || u.speed < speed {
                    speed = u.speed;
                }
            }
        }
        speed
    }

    /// Returns the total troop count by role.
    pub fn get_troop_count_by_role(&self, role: UnitRole) -> u32 {
        self.units
            .iter()
            .enumerate()
            .filter(move |(idx, quantity)| {
                if **quantity > 0 {
                    let unit = self.get_unit_by_idx(*idx as u8).unwrap();
                    return std::mem::discriminant(&unit.role) == std::mem::discriminant(&role);
                }
                false
            })
            .map(|(_, &q)| q)
            .sum()
    }

    /// Checks if the army contains only scouts (index 3) and no other units.
    pub fn is_only_scouts(&self) -> bool {
        self.units[3] > 0
            && self
                .units
                .iter()
                .enumerate()
                .all(|(idx, &count)| idx == 3 || count == 0)
    }

    /// Checks if the army contains catapults (index 7).
    pub fn has_catapults(&self) -> bool {
        self.units[7] > 0
    }

    /// Updates the units of the army.
    pub fn update_units(&mut self, units: &TroopSet) {
        self.units = *units;
    }

    pub fn add_unit(&mut self, name: UnitName, quantity: u32) -> Result<(), GameError> {
        if let Some(idx) = self.tribe.get_unit_idx_by_name(&name) {
            self.units[idx] += quantity;
            return Ok(());
        }

        Err(GameError::UnitNotFound(name))
    }

    /// Merges another army into the current one.
    pub fn merge(&mut self, other: &Army) -> Result<(), GameError> {
        if self.tribe != other.tribe {
            return Err(GameError::TribeMismatch);
        }

        for (idx, quantity) in other.units.iter().enumerate() {
            self.units[idx] += quantity;
        }

        Ok(())
    }

    // Returns the data information for a given unit in the army.
    fn get_unit_by_idx(&self, idx: u8) -> Option<Unit> {
        if idx.ge(&0) && idx.lt(&10) {
            Some(self.tribe.units()[idx as usize].clone())
        } else {
            None
        }
    }

    /// Returns the scouting points based on a given base points.
    fn scouting_points(&self, base_points: u8) -> u32 {
        let idx: usize = 3;
        let quantity = self.units[idx];
        let unit = self.get_unit_by_idx(idx as u8).unwrap();
        let smithy_improvement = self.apply_smithy_upgrade(&unit, idx, base_points as u32);
        smithy_improvement * quantity
    }

    /// Applies the smithy upgrade to a given combat value.
    fn apply_smithy_upgrade(&self, unit: &Unit, idx: usize, combat_value: u32) -> u32 {
        let level: i32 = self.smithy[idx].into();
        ((combat_value as f64)
            + ((combat_value + 300 * unit.cost.upkeep) as f64 / 7.0)
                * ((1.007f64).powi(level) - 1.0).floor()) as u32
    }
}

#[cfg(test)]
mod tests {
    use parabellum_types::tribe::Tribe;

    use crate::test_utils::{ArmyFactoryOptions, army_factory};

    #[test]
    fn test_army_upkeep() {
        // 10 Maceman (1 upkeep) + 5 Spearman (1 upkeep) = 15 upkeep
        let army = army_factory(ArmyFactoryOptions {
            tribe: Some(Tribe::Teuton),
            units: Some([10, 5, 0, 0, 0, 0, 0, 0, 0, 0]),
            ..Default::default()
        });

        assert_eq!(army.upkeep(), 15);

        // 10 Legionnaire (1 upkeep) + 5 Equites Imperatoris (3 upkeep) = 10 + 15 = 25 upkeep
        let army_roman = army_factory(ArmyFactoryOptions {
            tribe: Some(Tribe::Roman),
            units: Some([10, 0, 0, 0, 5, 0, 0, 0, 0, 0]),
            ..Default::default()
        });

        assert_eq!(army_roman.upkeep(), 25);
    }

    #[test]
    fn test_army_attack_points_no_smithy() {
        // 10 Maceman (40 attack) = 400 infantry
        // 5 Teutonic Knight (150 attack) = 750 infantry
        let (infantry, cavalry) = army_factory(ArmyFactoryOptions {
            tribe: Some(Tribe::Teuton),
            units: Some([10, 0, 0, 0, 0, 5, 0, 0, 0, 0]),
            smithy: Some([0; 8]), // No smithy upgrades
            ..Default::default()
        })
        .attack_points();

        assert_eq!(infantry, 400);
        assert_eq!(cavalry, 750);

        // 10 Legionnaire (40 attack) = 400 infantry
        // 5 Equites Imperatoris (120 attack) = 600 cavalry
        let (infantry, cavalry) = army_factory(ArmyFactoryOptions {
            tribe: Some(Tribe::Roman),
            units: Some([10, 0, 0, 0, 5, 0, 0, 0, 0, 0]),
            smithy: Some([0; 8]), // No smithy upgrades
            ..Default::default()
        })
        .attack_points();

        assert_eq!(infantry, 400);
        assert_eq!(cavalry, 600);
    }

    #[test]
    fn test_army_speed() {
        // Maceman (speed 14), Spearman (speed 14)
        let army_fast = army_factory(ArmyFactoryOptions {
            tribe: Some(Tribe::Teuton),
            units: Some([10, 5, 0, 0, 0, 0, 0, 0, 0, 0]),
            ..Default::default()
        });
        assert_eq!(army_fast.speed(), 14);

        // Maceman (speed 14), Ram (speed 8)
        let army_slow = army_factory(ArmyFactoryOptions {
            tribe: Some(Tribe::Teuton),
            units: Some([10, 0, 0, 0, 0, 0, 5, 0, 0, 0]),
            ..Default::default()
        });
        assert_eq!(army_slow.speed(), 8); // Speed is limited by the slowest unit (Ram)

        // No units
        let army_empty = army_factory(ArmyFactoryOptions {
            tribe: Some(Tribe::Teuton),
            units: Some([0; 10]),
            ..Default::default()
        });
        assert_eq!(army_empty.speed(), 0); // No units, speed is 0
    }
}
