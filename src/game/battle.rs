/// Battle
use serde::{Deserialize, Serialize};
use std::f64;

use crate::game::models::{
    ResourceGroup,
    army::{Army, TroopSet},
    buildings::{Building, BuildingName},
    village::{Village, VillageStocks},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub enum AttackType {
    Raid,   // Raid
    Normal, // Attack / Siege / Conquer
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub enum ScoutingTarget {
    Resources,
    Defenses,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub enum ScoutingTargetReport {
    Resources(ResourceGroup),
    Defenses {
        wall: Option<u8>,
        palace: Option<u8>,
        residence: Option<u8>,
    }, // wall level, residence level, palace level
}

// Represents the outcome of a battle
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScoutingBattleReport {
    // Indicates if defenders has detected the attack.
    // If true, defender will see a report as well.
    pub was_detected: bool,
    pub target: ScoutingTarget,
    pub target_report: ScoutingTargetReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingDamageReport {
    pub name: BuildingName,
    pub level_before: u8,
    pub level_after: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattlePartyReport {
    pub army_before: Army,
    pub survivors: TroopSet,
    pub losses: TroopSet,
    // hero_exp_gained: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleReport {
    pub attack_type: AttackType,
    pub attacker: BattlePartyReport,
    pub defender: Option<BattlePartyReport>,
    pub reinforcements: Vec<BattlePartyReport>, // Rinforzi

    pub scouting: Option<ScoutingBattleReport>,
    pub bounty: Option<ResourceGroup>,

    pub wall_damage: Option<BuildingDamageReport>,
    pub catapult_damage: Vec<BuildingDamageReport>,

    pub loyalty_before: u8,
    pub loyalty_after: u8,
}

pub struct Battle {
    attack_type: AttackType,
    attacker: Army,
    attacker_village: Village,
    defender_village: Village,
    catapult_targets: Option<[Building; 2]>,
}

impl Battle {
    pub fn new(
        attack_type: AttackType,
        attacker: Army,
        attacker_village: Village,
        defender_village: Village,
        catapult_targets: Option<[Building; 2]>,
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
    pub fn calculate_battle(&self) -> BattleReport {
        // STEP 1: Calculate total attack and defense points
        // 1.1: Attack points
        let (mut total_attacker_infantry_points, mut total_attacker_cavalry_points): (u32, u32) =
            self.attacker.attack_points();

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
        let mut total_defender_immensity: u32 = 0;

        let (mut total_defender_infantry_points, mut total_defender_cavalry_points): (u32, u32) =
            self.defender_village
                .army
                .clone()
                .map_or((0, 0), |a| a.defense_points());

        total_defender_immensity += self
            .defender_village
            .army
            .clone()
            .map_or(0, |a| a.immensity());

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

        // STEP 2: Calculate total power and casualties

        // 2.1 Total attack power
        let total_attack_power_f64 =
            1.0 + total_attacker_infantry_points as f64 + total_attacker_cavalry_points as f64;

        // 2.2: Total defense power
        let infantry_ratio_f64 = (total_attacker_infantry_points as f64) / total_attack_power_f64;
        let cavalry_ratio_f64 = (total_attacker_cavalry_points as f64) / total_attack_power_f64;

        let total_defense_power_f64 = (total_defender_infantry_points as f64 * infantry_ratio_f64)
            + (total_defender_cavalry_points as f64 * cavalry_ratio_f64);

        // 2.3: Morale bonus (if attacker pop > defender pop)
        let mut morale_bonus = 1.0;
        let total_attacker_population = self.attacker_village.population + self.attacker.upkeep();

        let defender_home_army_upkeep = self
            .defender_village
            .army
            .as_ref()
            .map_or(0, |a| a.upkeep());
        let total_defender_population = self.defender_village.population
            + defender_home_army_upkeep
            + self
                .defender_village
                .reinforcements
                .iter()
                .map(|a| a.upkeep())
                .sum::<u32>();

        if total_attacker_population > total_defender_population {
            let ratio = total_defender_population as f64 / total_attacker_population as f64;
            morale_bonus = ratio.powf(0.3); // TODO: simplified, original formula is more complex
        }

        let effective_attack_power_f64 = total_attack_power_f64 * morale_bonus;

        // 2.4: Combat formula
        let power_ratio: f64 = effective_attack_power_f64 / total_defense_power_f64;
        // Immensity
        let total_units_involved = self.attacker.immensity() + total_defender_immensity;

        let (attacker_loss_percentage, defender_loss_percentage) = calculate_losses_percentages(
            &self.attack_type,
            effective_attack_power_f64,
            total_defense_power_f64,
            total_units_involved,
        );

        let (attacker_survivors, attacker_losses) =
            self.attacker.calculate_losses(attacker_loss_percentage);

        let (defender_survivors, defender_losses) = self
            .defender_village
            .army
            .clone()
            .map_or(([0; 10], [0; 10]), |da| {
                da.calculate_losses(defender_loss_percentage)
            });

        let reinforcements_report: Vec<BattlePartyReport> = self
            .defender_village
            .reinforcements
            .iter()
            .map(|reinforcement| {
                let (survivors, losses) = reinforcement.calculate_losses(defender_loss_percentage);

                BattlePartyReport {
                    army_before: reinforcement.clone(),
                    survivors,
                    losses,
                }
            })
            .collect();

        // STEP 3: Calculate damages to wall and buildings

        let wall_level = self.defender_village.get_wall().map_or(0, |w| w.level);
        let mut wall_level_after = wall_level;

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
            wall_level_after = calculate_new_building_level(wall_level, ram_damage);
        }

        // 3.2: Catapults damage
        let surviving_catapults = attacker_survivors[7];
        let smithy_level: u8 = self.attacker.smithy[7];
        let mut buildings_levels: Option<[u8; 2]> = None;

        if self.attack_type == AttackType::Normal && surviving_catapults > 0 {
            let catapult_morale_bonus = (self.attacker_village.population as f64
                / total_defender_population as f64)
                .powf(0.3)
                .clamp(1.0, 3.0);

            // TODO: fix catapult targets (none, random, 1 or 2)
            let catapults_targets_quantity: u32 =
                match self.catapult_targets.clone().map_or(0, |cts| cts.len()) as u32 {
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

            if let Some(catapult_targets) = self.catapult_targets.clone() {
                buildings_levels = Some(
                    catapult_targets
                        .map(|target| calculate_new_building_level(target.level, catapult_damage)),
                );
            }
        }

        // STEP 4: Final result

        let wall_report = if wall_level > 0 {
            Some(BuildingDamageReport {
                name: self.defender_village.get_wall_name().unwrap(),
                level_before: wall_level,
                level_after: wall_level_after,
            })
        } else {
            None
        };

        let catapult_reports: Vec<BuildingDamageReport> = match buildings_levels {
            None => vec![],
            Some(levels) => levels
                .iter()
                .zip(self.catapult_targets.iter())
                .flat_map(|(&new_level, targets)| {
                    targets.clone().map(|target| BuildingDamageReport {
                        name: target.name.clone(),
                        level_before: target.level,
                        level_after: new_level,
                    })
                })
                .collect(),
        };

        let defender_report = if self.defender_village.army.is_some() {
            Some(BattlePartyReport {
                army_before: self.defender_village.army.clone().unwrap(),
                survivors: defender_survivors,
                losses: defender_losses,
            })
        } else {
            None
        };

        // bounty
        let bounty = calculate_bounty(
            self.attacker.bounty_capacity_troop_set(&attacker_survivors),
            &self.defender_village.stocks,
        );

        BattleReport {
            attack_type: self.attack_type.clone(),
            attacker: BattlePartyReport {
                army_before: self.attacker.clone(),
                survivors: attacker_survivors,
                losses: attacker_losses,
            },
            defender: defender_report,
            reinforcements: reinforcements_report,
            scouting: None, // it's not scouting
            bounty: Some(bounty),
            wall_damage: wall_report,
            catapult_damage: catapult_reports,
            loyalty_before: 100, // TODO: calculate loyalty
            loyalty_after: 100,  // TODO: calculate loyalty
        }
    }

    pub fn calculate_scout_battle(&self, target: ScoutingTarget) -> BattleReport {
        // STEP 1: Calculates attack and defense points for scouts

        let total_scout_attack_power = self.attacker.scouting_attack_points();
        let total_attack_scouts = self.attacker.unit_amount(3);

        let mut total_scout_defense_power = self
            .defender_village
            .army
            .clone()
            .map_or(0, |a| a.scouting_defense_points());
        let mut total_defense_scouts = 0;
        for reinforcement in self.defender_village.reinforcements.iter() {
            total_scout_defense_power += reinforcement.scouting_defense_points();
            total_defense_scouts += reinforcement.unit_amount(3);
        }

        let defender_has_scouts = total_scout_defense_power > 0;

        // STEP 2: Apply bonuses and casualties
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

        // STEP 3: Final result

        // Calculate casualties in attacker scouts
        let (attacker_survivors, attacker_losses) =
            self.attacker.calculate_losses(attacker_loss_percentage);

        let reinforcements_report: Vec<BattlePartyReport> = self
            .defender_village
            .reinforcements
            .iter()
            .map(|reinforcement| {
                BattlePartyReport {
                    army_before: reinforcement.clone(),
                    // No losses for defenders in scouting
                    survivors: reinforcement.units,
                    losses: [0; 10],
                }
            })
            .collect();

        // Prepare scouting target report
        let target_report = match target {
            ScoutingTarget::Resources => {
                let resources = self.defender_village.stocks.stored_resources();
                ScoutingTargetReport::Resources(resources)
            }
            ScoutingTarget::Defenses => {
                let wall = self.defender_village.get_wall().map(|w| w.level);

                let (palace, residence): (Option<u8>, Option<u8>) =
                    match self.defender_village.get_palace_or_residence() {
                        Some((b, BuildingName::Palace)) => (None, Some(b.level)),
                        Some((b, BuildingName::Residence)) => (None, Some(b.level)),
                        _ => (None, None),
                    };

                ScoutingTargetReport::Defenses {
                    wall,
                    palace,
                    residence,
                }
            }
        };

        BattleReport {
            attack_type: self.attack_type.clone(),
            attacker: BattlePartyReport {
                army_before: self.attacker.clone(),
                survivors: attacker_survivors,
                losses: attacker_losses,
            },
            defender: None,
            reinforcements: reinforcements_report,
            scouting: Some(ScoutingBattleReport {
                was_detected: defender_has_scouts,
                target,
                target_report,
            }),
            bounty: None, // No bounty for scouting
            wall_damage: None,
            catapult_damage: vec![],
            loyalty_before: 100, // TODO: calculate loyalty
            loyalty_after: 100,  // TODO: calculate loyalty
        }
    }
}

/// Calculates the bounty (loot) taken from the defender village, given the total
/// capacity of the surviving troops and the available stocks in the village.
/// The function distributes the loot proportionally among the resources,
/// taking into account the total capacity and the available resources,
///
/// # Arguments
/// * `total_capacity` - Total transport capacity from the actual TroopSet.
/// * `available_stocks` - Actual stocks from the defender village.
///
/// # Returns
/// * `ResourceGroup` - Stolen resources (Lumber, Clay, Iron, Crop).
fn calculate_bounty(total_capacity: u32, available_stocks: &VillageStocks) -> ResourceGroup {
    let available_lumber = available_stocks.lumber;
    let available_clay = available_stocks.clay;
    let available_iron = available_stocks.iron;
    let available_crop = available_stocks.crop.max(0) as u32;

    let total_available = available_lumber + available_clay + available_iron + available_crop;

    // If no resources availability, then no bounty.
    if total_capacity == 0 || total_available == 0 {
        return ResourceGroup::new(0, 0, 0, 0);
    }

    // Calculates the loot ratio.
    // Can't be > 1.0 (you can't take more than what's available)
    // Can't be > 1.0 (you can't take more than your capacity)
    // The combination is (capacity / resources).min(1.0)
    let loot_ratio = (total_capacity as f64 / total_available as f64).min(1.0);

    // Calculating initial proportional loot (rounded down)
    let mut bounty_lumber = (available_lumber as f64 * loot_ratio).floor() as u32;
    let mut bounty_clay = (available_clay as f64 * loot_ratio).floor() as u32;
    let mut bounty_iron = (available_iron as f64 * loot_ratio).floor() as u32;
    let mut bounty_crop = (available_crop as f64 * loot_ratio).floor() as u32;

    let total_looted = bounty_lumber + bounty_clay + bounty_iron + bounty_crop;

    // Calculate the "lost" capacity due to rounding
    let mut capacity_left = total_capacity - total_looted;

    // Distribute the remaining capacity, 1 by 1, in the order Lumber-Clay-Iron-Crop,
    // ensuring we do not take more than what is available.
    if capacity_left > 0 {
        let can_take_lumber = available_lumber - bounty_lumber;
        let take_lumber = capacity_left.min(can_take_lumber).min(1);
        bounty_lumber += take_lumber;
        capacity_left -= take_lumber;
    }

    if capacity_left > 0 {
        let can_take_clay = available_clay - bounty_clay;
        let take_clay = capacity_left.min(can_take_clay).min(1);
        bounty_clay += take_clay;
        capacity_left -= take_clay;
    }

    if capacity_left > 0 {
        let can_take_iron = available_iron - bounty_iron;
        let take_iron = capacity_left.min(can_take_iron).min(1);
        bounty_iron += take_iron;
        capacity_left -= take_iron;
    }

    if capacity_left > 0 {
        let can_take_crop = available_crop - bounty_crop;
        // At last, take all the possible
        let take_crop = capacity_left.min(can_take_crop);
        bounty_crop += take_crop;
    }

    ResourceGroup::new(bounty_lumber, bounty_clay, bounty_iron, bounty_crop)
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
    attack_power: f64,
    defense_power: f64,
    immensity: u32,
) -> (f64, f64) {
    let m_factor = calculate_m_factor(immensity);

    let attacker_loss_percentage: f64;
    let defender_loss_percentage: f64;

    if attack_power > defense_power {
        // Attacker wins
        (attacker_loss_percentage, defender_loss_percentage) = calculate_loss_factor_by_attack_type(
            attack_type,
            attack_power,
            defense_power,
            m_factor,
        );
    } else {
        // Defender wins (or draw)
        (defender_loss_percentage, attacker_loss_percentage) = calculate_loss_factor_by_attack_type(
            attack_type,
            defense_power,
            attack_power,
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

    4.0 * sigma(ad_ratio) * efficiency * upgrades / morale
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
    current_level
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Result,
        game::{
            models::{Player, Tribe},
            test_utils::*,
        },
    };

    #[test]
    fn test_losses_attacker_wins_normal() {
        let attack_power = 1000.0;
        let defense_power = 100.0;
        let immensity = 1100;
        let (atk_loss_perc, def_loss_perc) = calculate_losses_percentages(
            &AttackType::Normal,
            attack_power,
            defense_power,
            immensity,
        );

        assert_eq!(def_loss_perc, 1.0, "Defender should be wiped out");
        assert!(atk_loss_perc < 0.1, "Attacker losses should be minimal");
    }

    #[test]
    fn test_losses_defender_wins_normal() {
        let attack_power = 100.0;
        let defense_power = 1000.0;
        let immensity = 1100;
        let (atk_loss_perc, def_loss_perc) = calculate_losses_percentages(
            &AttackType::Normal,
            attack_power,
            defense_power,
            immensity,
        );

        assert_eq!(atk_loss_perc, 1.0, "Attacker should be wiped out");
        assert!(def_loss_perc < 0.1, "Defender losses should be minimal");
    }

    #[test]
    fn test_losses_equal_fight_raid() {
        let attack_power = 500.0;
        let defense_power = 500.0;
        let immensity = 1000;
        let (atk_loss_perc, def_loss_perc) =
            calculate_losses_percentages(&AttackType::Raid, attack_power, defense_power, immensity);

        assert!(
            (atk_loss_perc - 0.5).abs() < 0.001,
            "Attacker losses should be ~50%"
        );
        assert!(
            (def_loss_perc - 0.5).abs() < 0.001,
            "Defender losses should be ~50%"
        );
    }

    #[test]
    fn test_battle_100_legionaires_vs_10_spearmen() -> Result<()> {
        let (_attacker_player, attacker_village, attacker_army) =
            setup_party(Tribe::Roman, [100, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        let (_defender_player, mut defender_village, defender_home_army) =
            setup_party(Tribe::Teuton, [0, 10, 0, 0, 0, 0, 0, 0, 0, 0]);

        defender_village.army = Some(defender_home_army.clone());
        defender_village.update_state();

        let battle = Battle::new(
            AttackType::Normal,
            attacker_army.clone(),
            attacker_village,
            defender_village,
            None,
        );
        let report = battle.calculate_battle();
        let defender_report = report.defender.expect("Defender report should exist");

        let defender_losses: u32 = defender_report.losses.iter().sum();
        let initial_defender_troops: u32 = defender_home_army.immensity();
        assert_eq!(
            defender_losses, initial_defender_troops,
            "Defender should lose all {} troops (lost {})",
            initial_defender_troops, defender_losses
        );

        let defender_survivors: u32 = defender_report.survivors.iter().sum();
        assert_eq!(
            defender_survivors, 0,
            "Defender should not have survivors (survived {})",
            defender_survivors
        );

        let attacker_losses: u32 = report.attacker.losses.iter().sum();
        assert!(
            attacker_losses < 10 && attacker_losses > 0,
            "Attacker should lose very few troops (lost {})",
            attacker_losses
        );

        let attacker_survivors: u32 = report.attacker.survivors.iter().sum();
        let diff = attacker_army.immensity() - attacker_losses;
        assert_eq!(
            attacker_survivors, diff,
            "Attacker should have {} survivors {}",
            attacker_survivors, diff
        );

        Ok(())
    }

    fn setup_party(tribe: Tribe, units: TroopSet) -> (Player, Village, Army) {
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(tribe.clone()),
            ..Default::default()
        });
        let village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });
        let army = army_factory(ArmyFactoryOptions {
            player_id: Some(player.id),
            village_id: Some(village.id),
            tribe: Some(tribe),
            units: Some(units),
            ..Default::default()
        });

        (player, village, army)
    }
}
