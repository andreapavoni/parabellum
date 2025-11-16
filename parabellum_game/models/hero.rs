use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_core::{GameError, Result};
use parabellum_types::{
    army::Unit,
    common::{Cost, ResourceGroup},
    tribe::Tribe,
};

/// Resource bonus: Balanced (+3 for each) or focus on a single one (+10)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeroResourceFocus {
    Balanced,
    Wood,
    Clay,
    Iron,
    Crop,
}

impl Default for HeroResourceFocus {
    fn default() -> Self {
        Self::Balanced
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hero {
    pub id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub tribe: Tribe,

    // State
    /// Hero level (max 100).
    pub level: u16,
    /// Which resource to focus for production bonus.
    pub resource_focus: HeroResourceFocus,
    /// Hero health percentage.
    pub health: u16,
    /// Number of killed enemies crop consumption.
    pub experience: u32,

    /// Strength points (att+def).
    pub strength_points: u16,
    /// Off bonus % (+0.2% each point).
    pub off_bonus_points: u16,
    /// Def bonus % (+0.2% each point).
    pub def_bonus_points: u16,
    /// +5%/day each point.
    pub regeneration_points: u16,
    /// Production (+3 on all OR +10 on focused one).
    pub resources_points: u16,
    // Points to assign.
    pub unassigned_points: u16, // +5 points per level
}

impl Hero {
    /// Returns a new hero instance.
    pub fn new(
        id: Option<Uuid>,
        village_id: u32,
        player_id: Uuid,
        tribe: Tribe,
        points: Option<u16>,
    ) -> Self {
        Self {
            id: id.unwrap_or(Uuid::new_v4()),
            player_id,
            village_id,
            tribe: tribe.clone(),
            level: 0,
            experience: 0,
            health: 100,
            resource_focus: HeroResourceFocus::Balanced,
            strength_points: 0,
            off_bonus_points: 0,
            def_bonus_points: 0,
            regeneration_points: 0,
            resources_points: 0,
            unassigned_points: points.unwrap_or(0),
        }
    }

    /// Checks if hero is still alive.
    pub fn is_alive(&self) -> bool {
        self.health > 0
    }

    /// Offensive bonus % (0.2% per point).
    pub fn off_bonus(&self) -> f64 {
        1.0 + (self.off_bonus_points as f64) * 0.002
    }

    /// Defensive bonus % (0.2% per point).
    pub fn def_bonus(&self) -> f64 {
        1.0 + (self.def_bonus_points as f64) * 0.002
    }

    /// Hero combat strength (attack & defense).
    pub fn strength(&self) -> u32 {
        let base_strength = unit_strength(self.tribe.get_top_unit().unwrap());
        base_strength * (self.strength_points as u32)
    }

    /// Hourly production bonus for resources or focus.
    pub fn resources(&self) -> ResourceGroup {
        let points = self.resources_points as u32;
        match self.resource_focus {
            HeroResourceFocus::Balanced => {
                let res = 3 * points;
                ResourceGroup(res, res, res, res)
            }
            HeroResourceFocus::Wood => ResourceGroup(10 * points, 0, 0, 0),
            HeroResourceFocus::Clay => ResourceGroup(0, 10 * points, 0, 0),
            HeroResourceFocus::Iron => ResourceGroup(0, 0, 10 * points, 0),
            HeroResourceFocus::Crop => ResourceGroup(0, 0, 0, 10 * points),
        }
    }

    /// Regeneration %/day (10% base + 5% * regeneration points).
    /// Returns integer value representing a percentage.
    pub fn regeneration(&self) -> u16 {
        10u16 + 5u16 * self.regeneration_points
    }

    /// Total XP to get to next level (cap at 100).
    pub fn xp_for_next_level(&self) -> u32 {
        xp_for_level((self.level + 1).min(100))
    }

    /// Gain XP (1 XP for killed enemy crop consumption), handles autolevel + heal 100%.
    /// Returns gained levels.
    pub fn gain_experience(&mut self, gained: u32) -> u16 {
        if gained == 0 {
            return 0;
        }

        self.experience = self.experience.saturating_add(gained);
        let mut leveled = 0u16;
        loop {
            let need = self.xp_for_next_level();
            if self.experience < need {
                break;
            }
            self.level = (self.level + 1).min(100);
            self.unassigned_points = self.unassigned_points.saturating_add(5); // 5 points/level (T3)
            self.health = 100; // full heal on level up
            leveled += 1;
            if self.level == 100 {
                break;
            }
        }
        leveled
    }

    /// Assigns points (cap at 100) by consuming the unassigned points.
    pub fn assign_points(
        &mut self,
        strength: u16,
        off_bonus: u16,
        def_bonus: u16,
        regeneration: u16,
        resources: u16,
    ) -> Result<(), GameError> {
        let total = strength + off_bonus + def_bonus + regeneration + resources;
        if total > self.unassigned_points {
            return Err(GameError::NotEnoughHeroPoints);
        }

        // we could cap at 100 without failure, but that would mean that mistakenly assigned points will go to void
        let apply = |cur: &mut u16, add: u16| -> Result<(), GameError> {
            let new = *cur + add;
            if new > 100 {
                return Err(GameError::HeroAttributeOverflow);
            }
            *cur = new;
            Ok(())
        };

        apply(&mut self.strength_points, strength)?;
        apply(&mut self.off_bonus_points, off_bonus)?;
        apply(&mut self.def_bonus_points, def_bonus)?;
        apply(&mut self.regeneration_points, regeneration)?;
        apply(&mut self.resources_points, resources)?;

        self.unassigned_points -= total;
        Ok(())
    }

    /// Battle damages: if loss_ratio >= 0.9 → death; else (loss_ratio * 100) HP
    pub fn apply_battle_damage(&mut self, loss_ratio: f64) {
        if loss_ratio >= 0.9 {
            self.health = 0;
            return;
        }

        let damage = (loss_ratio.clamp(0.0, 1.0) * 100.0).round() as i32;
        let current_health = (self.health as i32 - damage).max(0) as u16;
        self.health = current_health;
    }

    /// Daily regeneration tick: 10% base + 5% for each regeneration point (cap 100).
    pub fn daily_regeneration_tick(&mut self) {
        if !self.is_alive() {
            return;
        }
        let base = 10.0;
        let extra = 5.0 * (self.regeneration_points as f32);
        let heal = (base + extra).min(100.0).round() as i32;
        self.health = (self.health as i32 + heal).min(100) as u16;
    }

    /// Revival costs (based on tribe's top unit costs; time calculated in T4 style)
    pub fn resurrection_cost(&self, server_speed: i8) -> Cost {
        let base_cost = &self.tribe.get_top_unit().unwrap().cost;
        let mult = revive_cost_multiplier(self.level);
        let resources = base_cost.resources.clone() * mult;

        // Revival time: min(level+1, 24) hours / (floor(speed/3)+1)
        let speed = server_speed.max(1) as f64;
        let adjusted_speed = (speed / 3.0) + 1.0;
        let hours = ((self.level + 1).min(24) as f64 / adjusted_speed).max(1.0);
        let time = (hours * 3600.0).round() as u32;

        Cost {
            resources,
            upkeep: base_cost.upkeep,
            time,
        }
    }

    /// Resurrects an hero.
    pub fn resurrect(&mut self, village_id: u32, reset: bool) {
        self.health = 100;
        self.village_id = village_id;
        if reset {
            self.level = 0;
            self.unassigned_points = 5;
            self.experience = 0;
            self.def_bonus_points = 0;
            self.off_bonus_points = 0;
            self.regeneration_points = 0;
            self.resources_points = 0;
            self.strength_points = 0;
            self.resource_focus = HeroResourceFocus::Balanced;
        };
    }
}

/// Total XP to get to a specific level [T3: 50*(L^2+L)]
fn xp_for_level(level: u16) -> u32 {
    // default T3
    (50u32)
        .saturating_mul(level as u32)
        .saturating_mul(level as u32 + 1)
}

/// Revival multiplier based on level.
fn revive_cost_multiplier(level: u16) -> f64 {
    2.0 * (1.0 + level as f64).powf(1.6)
}

/// Returns the highest combat value between attack, cavalry-def or infantry-def of a unit.
fn unit_strength(unit: &Unit) -> u32 {
    unit.attack
        .max(unit.defense_infantry)
        .max(unit.defense_cavalry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use parabellum_core::Result;

    fn setup() -> Result<Hero> {
        let hero = Hero::new(None, 1, Uuid::new_v4(), Tribe::Roman, Some(5));
        Ok(hero)
    }

    #[test]
    fn test_off_bonus() -> Result<()> {
        let mut hero = setup()?;
        hero.unassigned_points = 150;

        hero.assign_points(0, 10, 0, 0, 0)?;
        assert_eq!(hero.off_bonus(), 1.02);

        hero.assign_points(0, 90, 0, 0, 0)?;
        assert_eq!(hero.off_bonus(), 1.2);

        Ok(())
    }

    #[test]
    fn test_def_bonus() -> Result<()> {
        let mut hero = setup()?;
        hero.unassigned_points = 150;

        hero.assign_points(0, 0, 10, 0, 0)?;
        assert_eq!(hero.def_bonus(), 1.02);

        hero.assign_points(0, 0, 90, 0, 0)?;
        assert_eq!(hero.def_bonus(), 1.2);
        Ok(())
    }

    #[test]
    fn test_strength() -> Result<()> {
        let mut hero = setup()?;
        hero.unassigned_points = 150;

        // 180 is the strenght from Equites Caesaris higher combat value
        hero.assign_points(10, 0, 0, 0, 0)?;
        assert_eq!(hero.strength(), 1800);

        hero.assign_points(90, 0, 0, 0, 0)?;
        assert_eq!(hero.strength(), 18000);
        Ok(())
    }

    #[test]
    fn test_resources() -> Result<()> {
        let mut hero = setup()?;
        hero.unassigned_points = 150;

        hero.resource_focus = HeroResourceFocus::Balanced;
        hero.assign_points(0, 0, 0, 0, 10)?;
        assert_eq!(hero.resources(), ResourceGroup(30, 30, 30, 30));

        hero.resource_focus = HeroResourceFocus::Wood;
        assert_eq!(hero.resources(), ResourceGroup(100, 0, 0, 0));

        hero.resource_focus = HeroResourceFocus::Clay;
        assert_eq!(hero.resources(), ResourceGroup(0, 100, 0, 0));

        hero.resource_focus = HeroResourceFocus::Iron;
        assert_eq!(hero.resources(), ResourceGroup(0, 0, 100, 0));

        hero.resource_focus = HeroResourceFocus::Crop;
        assert_eq!(hero.resources(), ResourceGroup(0, 0, 0, 100));

        hero.resource_focus = HeroResourceFocus::Balanced;
        hero.assign_points(0, 0, 0, 0, 90)?;
        assert_eq!(hero.resources(), ResourceGroup(300, 300, 300, 300));
        Ok(())
    }

    #[test]
    fn test_regeneration() -> Result<()> {
        let mut hero = setup()?;
        hero.unassigned_points = 150;

        hero.assign_points(0, 0, 0, 10, 0)?;
        assert_eq!(hero.regeneration(), 60);

        hero.assign_points(0, 0, 0, 90, 0)?;
        assert_eq!(hero.regeneration(), 510);
        Ok(())
    }

    #[test]
    fn xp_and_leveling_uses_t3_curve() -> Result<()> {
        let mut hero = setup()?;

        assert_eq!(hero.level, 0);
        assert_eq!(hero.unassigned_points, 5);
        // T3 cumulative: level 1 total = 100; level 2 total = 300
        let gained1 = hero.gain_experience(100);
        assert_eq!(gained1, 1);
        assert_eq!(hero.level, 1);
        assert_eq!(hero.health, 100);
        assert_eq!(hero.unassigned_points, 10); // 5 start + 5 on level up

        let gained2 = hero.gain_experience(200); // reach 300
        assert_eq!(gained2, 1);
        assert_eq!(hero.level, 2);
        assert_eq!(hero.unassigned_points, 15);
        Ok(())
    }

    #[test]
    fn battle_damage_and_death_threshold() -> Result<()> {
        let mut hero = setup()?;
        hero.health = 100;
        hero.apply_battle_damage(0.5); // -50 hp
        assert_eq!(hero.health, 50);
        hero.apply_battle_damage(0.89);
        assert_eq!(hero.health, 0.max(50 - 89) as u16); // clamps to 0 if negative?
        // reset and test hard death
        hero.health = 100;
        hero.apply_battle_damage(0.9); // death
        assert_eq!(hero.health, 0);
        assert!(!hero.is_alive());
        Ok(())
    }

    #[test]
    fn daily_regen_base_plus_points() -> Result<()> {
        let mut hero = setup()?;
        hero.assign_points(0, 0, 0, 2, 0)?;
        hero.health = 10;
        // base 10% + 5%*regen(=2) = 20% HP heal
        hero.daily_regeneration_tick();
        assert_eq!(hero.health, 30);
        Ok(())
    }

    #[test]
    fn resource_bonus_balanced_and_focus() -> Result<()> {
        let mut hero = setup()?;

        hero.assign_points(0, 0, 0, 0, 4)?;
        hero.resource_focus = HeroResourceFocus::Balanced;
        let b = hero.resources();
        assert_eq!(b.lumber(), 12);
        assert_eq!(b.clay(), 12);
        assert_eq!(b.iron(), 12);
        assert_eq!(b.crop(), 12);

        hero.resource_focus = HeroResourceFocus::Iron;
        let b = hero.resources();
        assert_eq!(b.lumber(), 0);
        assert_eq!(b.iron(), 40);
        Ok(())
    }

    #[test]
    fn resurrection_cost_scaling() -> Result<()> {
        let mut hero = setup()?;

        // L0: mult ≈ 2x
        hero.level = 0;
        let c0 = hero.resurrection_cost(1);
        assert_eq!(c0.resources.lumber(), 550 * 2);
        assert_eq!(c0.time, 3600); // (0+1)h @speed1

        // L2: mult ≈ 11.6x
        hero.level = 2;
        let c2 = hero.resurrection_cost(1);
        assert!(c2.resources.lumber() >= 550 * 11 && c2.resources.lumber() <= 550 * 12); // ~11x..12x
        assert_eq!(c2.time, (2.25 * 3600.0) as u32);

        // speed-3 server halves via denom=(floor(3/3)+1)=2 → 1.5h for L2
        let cf = hero.resurrection_cost(3);
        assert_eq!(cf.time, (1.5f32 * 3600.0) as u32);
        Ok(())
    }

    #[test]
    fn assign_points_caps_and_budget() -> Result<()> {
        let mut hero = setup()?;
        hero.unassigned_points = 3;
        // ok
        hero.assign_points(1, 1, 1, 0, 0)?;
        assert_eq!(hero.unassigned_points, 0);
        // fail on budget
        assert!(hero.assign_points(1, 0, 0, 0, 0).is_err());
        // fail on cap
        hero.strength_points = 100;
        assert!(hero.assign_points(1, 0, 0, 0, 0).is_err());
        Ok(())
    }
}
