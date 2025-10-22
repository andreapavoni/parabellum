/// Battle
use serde::{Deserialize, Serialize};
use std::f64;

use crate::game::models::{army::Army, buildings::Building, village::Village};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub enum AttackType {
    Raid,   // Raid
    Normal, // Attack / Siege / Conquer
}

// Represents the outcome of a battle
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BattleResult {
    // ... more data like hero health, etc.
    pub attacker_loss_percentage: f64,
    pub defender_loss_percentage: f64,
    pub wall_level: u8,
    pub buildings_damages: [u8; 2], // damaged buildings
    pub bounty: u32,
    pub loyalty: u16,
}

pub struct ScoutBattleResult {
    // Indicates if defenders has detected the attack.
    // If true, defender will see a report as well.
    pub was_detected: bool,
    pub attacker_loss_percentage: f64,
}

pub struct Battle {
    attack_type: AttackType,
    attacker: Army,
    attacker_village: Village,
    defender_village: Village,
    catapult_targets: [Building; 2],
}

impl Battle {
    pub fn new(
        attack_type: AttackType,
        attacker: Army,
        attacker_village: Village,
        defender_village: Village,
        catapult_targets: [Building; 2],
    ) -> Self {
        Self {
            attack_type,
            attacker,
            attacker_village,
            defender_village,
            catapult_targets,
        }
    }

    // Main function to calculate the battle
    pub fn calculate_battle(&self) -> BattleResult {
        // ====================================================================
        // STEP 1: Calculate total attack and defense points
        // ====================================================================
        let mut total_attacker_infantry_points: u32;
        let mut total_attacker_cavalry_points: u32;

        (
            total_attacker_infantry_points,
            total_attacker_cavalry_points,
        ) = self.attacker.attack_points();

        // 1.1: Attack points

        // 1.2: Hero attack bonus
        if let Some(hero) = &self.attacker.hero {
            let hero_bonus_multiplier = 1.0 + (hero.get_attack_bonus(true) as f64 / 100.0);
            total_attacker_infantry_points =
                (total_attacker_infantry_points as f64 * hero_bonus_multiplier) as u32;
            total_attacker_cavalry_points =
                (total_attacker_cavalry_points as f64 * hero_bonus_multiplier) as u32;
            // Hero attack points
            total_attacker_infantry_points += hero.attack_points;
        }

        // 1.3: Defense points (village troops + reinforcements)
        let mut total_defender_infantry_points: u32;
        let mut total_defender_cavalry_points: u32;
        let mut total_defender_immensity: u32 = 0;

        (
            total_defender_infantry_points,
            total_defender_cavalry_points,
        ) = match self.defender_village.army.clone() {
            Some(army) => army.defense_points(),
            None => (0, 0),
        };

        total_defender_immensity += match self.defender_village.army.clone() {
            Some(army) => army.immensity(),
            None => 0,
        };

        for defender_army in self.defender_village.reinforcements.iter() {
            let (defender_infantry_points, defender_cavalry_points) =
                defender_army.defense_points();

            total_defender_infantry_points += defender_infantry_points;
            total_defender_cavalry_points += defender_cavalry_points;
            total_defender_immensity += defender_army.immensity();

            // Apply hero defense bonus
            if let Some(hero) = &defender_army.hero {
                let hero_bonus_multiplier = 1.0 + (hero.get_defense_bonus() as f64 / 100.0);
                total_defender_infantry_points =
                    (total_defender_infantry_points as f64 * hero_bonus_multiplier) as u32;
                total_defender_cavalry_points =
                    (total_defender_cavalry_points as f64 * hero_bonus_multiplier) as u32;
                total_defender_infantry_points += hero.defense_points;
                total_defender_cavalry_points += hero.defense_points;
            }
        }

        // 1.4: Calculate village defense bonus
        // Resindence/Palace
        if let Some((residence_palace, _)) = self.defender_village.get_palace_or_residence() {
            let bonus = 2.0 * (residence_palace.level as f64).powi(2);
            total_defender_infantry_points = (total_defender_infantry_points as f64 + bonus) as u32;
            total_defender_cavalry_points = (total_defender_cavalry_points as f64 + bonus) as u32;
        }

        // Wall
        let wall_bonus = self.defender_village.get_wall_defense_bonus();

        total_defender_infantry_points =
            (total_defender_infantry_points as f64 * wall_bonus) as u32;
        total_defender_cavalry_points = (total_defender_cavalry_points as f64 * wall_bonus) as u32;

        // ====================================================================
        // STEP 2: Calculate total power and casualties
        // ====================================================================

        // 2.1 Total attack power
        let total_attack_power = 1 + total_attacker_infantry_points + total_attacker_cavalry_points;

        // 2.2: Total defense power
        let infantry_ratio = total_attacker_infantry_points / total_attack_power;
        let cavalry_ratio = total_attacker_cavalry_points / total_attack_power;
        let total_defense_power = (total_defender_infantry_points * infantry_ratio)
            + (total_defender_cavalry_points * cavalry_ratio);

        // 2.3: Morale bonus (if attacker pop > defender pop)
        let mut morale_bonus = 1.0;
        let total_defender_population = self.defender_village.population
            + self
                .defender_village
                .reinforcements
                .iter()
                .map(|a| a.upkeep())
                .sum::<u32>();

        if self.attacker_village.population > total_defender_population {
            let ratio = total_defender_population as f64 / self.attacker_village.population as f64;
            morale_bonus = ratio.powf(0.2); // TODO: simplified, original formula is more complex
        }

        let effective_attack_power = (total_attack_power as f64 / morale_bonus) as u32;

        // 2.4: Combat formula
        let power_ratio: f64 = effective_attack_power as f64 / total_defense_power as f64;
        // Immensity
        let total_units_involved = self.attacker.immensity() + total_defender_immensity;

        let (attacker_loss_percentage, defender_loss_percentage) = calculate_losses_percentages(
            &self.attack_type,
            effective_attack_power,
            total_defense_power,
            total_units_involved,
        );

        let (attacker_survivors, _attacker_losses) =
            self.attacker.calculate_losses(attacker_loss_percentage);

        // ====================================================================
        // STEP 3: Calculate damages to wall and buildings
        // ====================================================================
        let mut wall_level = match self.defender_village.get_wall() {
            Some(wall) => wall.level,
            _ => 0,
        };

        // Variables to calculate buildings damages
        let buildings_durability = self.defender_village.get_buildings_durability();

        // 3.1: Rams damage
        let surviving_rams = attacker_survivors[6];
        let smithy_level: u8 = self.attacker.smithy[6];
        if surviving_rams > 0 && wall_level > 0 {
            let ram_damage = calculate_machine_damage(
                surviving_rams,
                smithy_level,
                buildings_durability,
                power_ratio,
                1.0, // Morale for rams is 1.0
            );
            wall_level = calculate_new_building_level(wall_level, ram_damage);
        }

        // 3.2: Catapults damage
        let surviving_catapults = attacker_survivors[7];
        let smithy_level: u8 = self.attacker.smithy[7];
        let mut buildings_levels: [u8; 2] = [0; 2];

        if self.attack_type == AttackType::Normal && surviving_catapults > 0 {
            let catapult_morale_bonus = (self.attacker_village.population as f64
                / total_defender_population as f64)
                .powf(0.3)
                .clamp(1.0, 3.0);

            // TODO: fix catapult targets (none, random, 1 or 2)
            let catapults_targets_quantity: u32 = match self.catapult_targets.len() as u32 {
                0 => 1,
                len => len,
            };

            let catapult_damage = calculate_machine_damage(
                surviving_catapults / catapults_targets_quantity, // Quantità per questo target
                smithy_level,
                buildings_durability,
                power_ratio,
                catapult_morale_bonus,
            );

            buildings_levels = self
                .catapult_targets
                .clone()
                .map(|target| calculate_new_building_level(target.level, catapult_damage));
        }

        // ====================================================================
        // STEP 4: Final result
        // ====================================================================

        BattleResult {
            attacker_loss_percentage,
            defender_loss_percentage,
            wall_level,
            buildings_damages: buildings_levels,
            bounty: 0,
            loyalty: 100,
        }
    }

    pub fn calculate_scout_battle(&self) -> ScoutBattleResult {
        // ====================================================================
        // STEP 1: Calculates attack and defense points for scouts
        // ====================================================================

        let total_scout_attack_power = self.attacker.scouting_attack_points();
        let total_attack_scouts = self.attacker.unit_amount(3);

        let mut total_scout_defense_power = match self.defender_village.army.clone() {
            Some(army) => army.scouting_defense_points(),
            None => 0,
        };
        let mut total_defense_scouts = 0;
        for reinforcement in self.defender_village.reinforcements.iter() {
            total_scout_defense_power += reinforcement.scouting_defense_points();
            total_defense_scouts += reinforcement.unit_amount(3);
        }

        let defender_has_scouts = total_scout_defense_power > 0;

        // ====================================================================
        // STEP 2: Apply bonuses and casualties
        // ====================================================================
        let wall_bonus = self.defender_village.get_wall_defense_bonus();

        // 2.1: Apply wall defense bonus
        total_scout_defense_power = (total_scout_defense_power as f64 * wall_bonus) as u32 + 10;

        let mut attacker_loss_percentage = 0.0;

        // 2.2: Check conditions for attacker casualties
        // Egiptian tribe is immune to getting detected when scouting
        // let attacker_is_immune = self.attacker.tribe == 5;

        if defender_has_scouts && total_scout_attack_power > 0 {
            // If there are defenders and attacker isn't immune, calculate casualties
            let total_units_involved = total_attack_scouts + total_defense_scouts;
            let m_factor = calculate_m_factor(total_units_involved);

            // Morale factor should be applies here as well, but in the original PHP is hidden
            // Skip this for simiplicity, but sit should be calculated before
            let power_ratio = total_scout_defense_power as f64 / total_scout_attack_power as f64;

            // Original formula is `min(1, (defense/attack)^M)`.
            attacker_loss_percentage = power_ratio.powf(m_factor).min(1.0);
        }

        // ====================================================================
        // STEP 3: Final result
        // ====================================================================

        // Calculate casualties in attacker scouts
        let (_attacker_survivors, _attacker_losses) =
            self.attacker.calculate_losses(attacker_loss_percentage);

        return ScoutBattleResult {
            was_detected: defender_has_scouts,
            attacker_loss_percentage,
        };
    }
}

// Massive battles factor (Mfactor)
fn calculate_m_factor(immensity: u32) -> f64 {
    if immensity >= 1000 {
        (2.0 * (1.8592 - (immensity as f64).powf(0.015))).clamp(1.2578, 1.5)
    } else {
        1.5
    }
}

// Losses are calculated in percentages and applied to all armies involved, according to a winner/loser logic
fn calculate_losses_percentages(
    attack_type: &AttackType,
    attack_power: u32,
    defense_power: u32,
    immensity: u32,
) -> (f64, f64) {
    let m_factor = calculate_m_factor(immensity);

    let attacker_loss_percentage: f64;
    let defender_loss_percentage: f64;

    if attack_power > defense_power {
        // Attacker wins
        (attacker_loss_percentage, defender_loss_percentage) = calculate_loss_factor_by_attack_type(
            attack_type,
            attack_power as f64,
            defense_power as f64,
            m_factor,
        );
    } else {
        // Defender wins (or draw)
        (defender_loss_percentage, attacker_loss_percentage) = calculate_loss_factor_by_attack_type(
            attack_type,
            defense_power as f64,
            attack_power as f64,
            m_factor,
        );
    }

    (attacker_loss_percentage, defender_loss_percentage)
}

// Loss factor is calculated based on the attack type
fn calculate_loss_factor_by_attack_type(
    attack_type: &AttackType,
    winner: f64,
    loser: f64,
    m_factor: f64,
) -> (f64, f64) {
    let loss_factor = (loser / winner).powf(m_factor);
    let winner_losses: f64;
    let loser_losses: f64;

    match attack_type {
        AttackType::Raid => {
            winner_losses = loss_factor / (1.0 + loss_factor);
            loser_losses = 1.0 / (1.0 + loss_factor);
        }

        AttackType::Normal => {
            winner_losses = loss_factor;
            loser_losses = 1.0;
        }
    }

    (winner_losses, loser_losses)
}

// sigma function from Kirilloid to calculate damages to buildings (catapults) and wall (rams)
// $this->sigma = function($x) { return ($x > 1 ? 2 - $x ** -1.5 : $x ** 1.5) / 2; };
fn sigma(x: f64) -> f64 {
    if x > 1.0 {
        (2.0 - x.powf(-1.5)) / 2.0
    } else {
        x.powf(1.5) / 2.0
    }
}

// Calculates damage for catapults/rams
fn calculate_machine_damage(
    quantity: u32,
    smithy_level: u8,
    durability: f64,
    ad_ratio: f64,
    morale: f64,
) -> f64 {
    let upgrades = 1.0205f64.powf(smithy_level as f64); // original formula is pow(1.0205, level)
    let efficiency = (quantity as f64 / durability).floor();

    return 4.0 * sigma(ad_ratio) * efficiency * upgrades / morale;
}

// Calculates new building level after damages
fn calculate_new_building_level(old_level: u8, mut damage: f64) -> u8 {
    let mut current_level = old_level;
    damage -= 0.5;
    if damage < 0.0 {
        return current_level;
    }

    while damage >= current_level as f64 && current_level > 0 {
        damage -= current_level as f64;
        current_level -= 1;
    }
    return current_level;
}
