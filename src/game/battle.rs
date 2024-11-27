use rand::Rng;

use super::models::{
    army::Army,
    buildings::{Building, BuildingName},
    common::Tribe,
    village::Village,
};

#[derive(Debug, Clone, Default)]
pub struct CataTargets(pub Option<BuildingName>, pub Option<BuildingName>);

impl CataTargets {
    pub fn targets(&self) -> Vec<BuildingName> {
        let mut targets: Vec<BuildingName> = vec![];

        if let Some(t) = self.0.clone() {
            targets.push(t);
        }

        if let Some(t) = self.1.clone() {
            targets.push(t);
        }

        targets
    }
}

#[derive(Debug, Clone, Default)]
struct BattleState {
    atk_won: bool,
    atk_points: u64,
    def_points: u64,
    winner_points: u64,
    loser_points: u64,
    immensity_factor: f64,
    winner_losses_percent: f64,
    loser_losses_percent: f64,
    reinforcement_losses_percent: f64,
}

#[derive(Debug, Clone)]
pub struct Battle {
    pub attacker_army: Army,
    pub attacker_village: Village,
    pub defender_village: Village,
    pub is_normal: bool,
    pub is_scouting: bool,
    pub cata_targets: CataTargets,
    state: BattleState,
}

impl Battle {
    pub fn new<'b>(
        attacker_army: Army,
        attacker_village: Village,
        defender_village: Village,
        is_normal: bool,
        is_scouting: bool,
        cata_targets: CataTargets,
    ) -> Self {
        Self {
            attacker_army,
            attacker_village,
            defender_village,
            is_normal,
            is_scouting,
            cata_targets,
            state: Default::default(),
        }
    }

    // Calculates a battle between two armies. Kirilloid's formulas.
    pub fn combat(&mut self) {
        self.calculate_battle_points();

        if !self.is_scouting && self.is_normal {
            self.apply_catapults_damage();
            self.apply_rams_damage();
        }

        // Recalculate battle points and bonuses after rams have passed
        self.calculate_battle_points();

        self.apply_defender_morale_bonus();

        self.calculate_outcome();

        self.calculate_immensity_factor();
        self.calculate_losses_percent();
        self.apply_losses();
    }

    // Calculates attacker and defender points, including Smithy upgrades and bonuses.
    fn calculate_battle_points(&mut self) {
        if self.is_scouting {
            // Calculate the total spy and counterspy power.
            self.state.atk_points = self.attacker_army.scouting_attack_points();
            self.state.def_points = self.defender_village.army.scouting_defense_points();

            // Include reinforcements power
            for r in self.defender_village.reinforcements.clone() {
                self.state.def_points += r.scouting_defense_points()
            }
        }

        // Calculate the Cavalry Attacking Power (CAP) and Infantry Attacking Power (IAP)
        let cavalry_atk_points: u64;
        // Calculate the Defensive Power against Cavalry (CDP) and Defensive Power
        // against Infantry (IDP)
        let infantry_atk_points: u64;
        (infantry_atk_points, cavalry_atk_points) = self.attacker_army.attack_points();

        let mut cavalry_def_points: u64;
        let mut infantry_def_points: u64;
        (infantry_def_points, cavalry_def_points) = self.defender_village.army.defense_points();

        // Include reinforcements defensive power
        for r in self.defender_village.reinforcements.clone() {
            let rcdp: u64;
            let ridp: u64;
            (ridp, rcdp) = r.defense_points();
            infantry_def_points += ridp;
            cavalry_def_points += rcdp;
        }

        // Calculate the total offensive and defensive power.
        self.state.atk_points = infantry_atk_points + cavalry_atk_points;
        let cavalry_atk_percent: f64 = cavalry_atk_points as f64 / self.state.atk_points as f64;
        let infantry_atk_percent: f64 = infantry_atk_points as f64 / self.state.atk_points as f64;

        self.state.def_points = (infantry_def_points as f64 * infantry_atk_percent
            + cavalry_def_points as f64 * cavalry_atk_percent)
            .floor() as u64;

        // Each village has a basic defense value of 10
        self.state.def_points += 10;

        // Battle bonuses: order matters!
        self.apply_palace_defense();
        self.apply_wall_bonus();
    }

    // Palace/Residence have some defense value
    fn apply_palace_defense(&mut self) {
        self.state.def_points += match self.defender_village.get_palace_or_residence() {
            Some((building, _)) => (building.level * building.level) as u64 * 2,
            None => 0 as u64,
        };
    }

    // Walls give a percentual bonus to total defense.
    fn apply_wall_bonus(&mut self) {
        if let Some(wall) = self.defender_village.get_wall() {
            let tribe_bonus: f64 = match self.defender_village.tribe {
                Tribe::Roman => 1.030,
                Tribe::Gaul => 1.025,
                Tribe::Teuton => 1.020,
                _ => 1.0,
            };

            let bonus = tribe_bonus.powf(wall.level as f64);

            self.state.def_points = (self.state.def_points as f64 * bonus).floor() as u64
        }

        // bonus := math.Pow(tribeBonus, float64(level))
        // b.state.defPoints = uint64(math.Floor(float64(b.state.defPoints) * bonus))
    }

    // Morale changes when defender's account population is lower than attacker's one.
    fn apply_defender_morale_bonus(&mut self) {
        let atk_pop = self.attacker_village.population;
        let def_pop = self.defender_village.population;

        let mut bonus = (atk_pop as f64 / def_pop as f64).powf(0.2).floor();

        // Special case: if attacker has fewer points than defender (including all bonuses), and has more population, formula changes:
        if self.state.atk_points < self.state.def_points {
            bonus = (atk_pop as f64 / def_pop as f64)
                .powf(0.2 * (self.state.atk_points as f64 / self.state.def_points as f64));
        }

        // Morale bonus never goes beyond +50% regardless of defender's population
        if bonus > 1.5 {
            bonus = 1.5
        }

        self.state.def_points = (self.state.def_points as f64 * bonus) as u64;
    }

    // Determine winner and loser of this battle.
    fn calculate_outcome(&mut self) {
        // A single unit with less than 83 attack power will always die regardless of defenses
        let lone_attack = self.attacker_army.immensity() == 1 && self.state.atk_points < 83;

        // Determine the winner and loser of the battle
        if self.state.atk_points >= self.state.def_points && !lone_attack {
            // Attacker wins
            self.state.atk_won = true;
            self.state.winner_points = self.state.atk_points;
            self.state.loser_points = self.state.def_points;
        } else {
            // Defender wins
            self.state.atk_won = false;
            self.state.winner_points = self.state.def_points;
            self.state.loser_points = self.state.atk_points;
        }
    }

    // Calculates the total number of troops involved in the battle.
    fn calculate_immensity_factor(&mut self) {
        let immensity =
            (self.attacker_army.immensity() + self.defender_village.army.immensity()) as f64; // + self.defender_village.reinforcements.immensity();
        if self.is_scouting || immensity < 1000.0 {
            self.state.immensity_factor = 1.5;
            return;
        }

        self.state.immensity_factor = 2.0 * (1.8592f64 - immensity.powf(0.015))
    }

    // Calculates the losses percentuals of both sides.
    fn calculate_losses_percent(&mut self) {
        self.state.winner_losses_percent = (self.state.loser_points as f64
            / self.state.winner_points as f64)
            .powf(self.state.immensity_factor)
            / 100.0;

        // in normal attacks, loser loses everything
        if self.is_normal {
            self.state.loser_losses_percent = 100.0;

            // in case of spying and defender has lost, it won't lose any troop
            if self.is_scouting && self.state.atk_won {
                self.state.loser_losses_percent = 0.0;
            }
            return;
        }

        // for raid attacks
        self.state.winner_losses_percent =
            self.state.winner_losses_percent / (100.0 + self.state.winner_losses_percent);
        self.state.loser_losses_percent = 100.0 - self.state.winner_losses_percent
    }

    // Apply the losses percentuals on both armies.
    fn apply_losses(&mut self) {
        if self.state.atk_won {
            self.attacker_army
                .apply_losses(self.state.winner_losses_percent);
            self.defender_village
                .army
                .apply_losses(self.state.loser_losses_percent);
            self.state.reinforcement_losses_percent = self.state.loser_losses_percent;
        } else {
            self.attacker_army
                .apply_losses(self.state.loser_losses_percent);
            self.defender_village
                .army
                .apply_losses(self.state.loser_losses_percent);
            self.state.reinforcement_losses_percent = self.state.winner_losses_percent;
        }

        let mut reinforcements: Vec<Army> = vec![];

        for r in self.defender_village.reinforcements.clone() {
            let mut updated = r.clone();
            updated.apply_losses(self.state.reinforcement_losses_percent);
            reinforcements.push(updated);
        }

        self.defender_village.reinforcements = reinforcements;
    }

    // Catas and rams

    // Applies damage to buildings when hit by catapults.
    fn apply_catapults_damage(&mut self) {
        let working_catas = self.get_working_siege_units(self.attacker_army.unit_amount(7));
        if working_catas <= 0 {
            return;
        }
        let morale = self.get_siege_morale();
        let cata_smithy = self.attacker_army.smithy[7];
        let buildings_durability = self.defender_village.get_buildings_durability();

        let mut atk_rally_point = 0;
        if let Some(rally_point) = self
            .attacker_village
            .get_building_by_name(BuildingName::RallyPoint)
        {
            atk_rally_point = rally_point.level as i32;
        }

        // To hit 2 targets we need at least 20 catapults, 1 target otherwise
        // To choose 1 target we needRallyPoint lvl 10 or above
        // To choose 2 targets we needRallyPoint lvl 20
        match (atk_rally_point, working_catas) {
            (20, 20..) => (),
            (10..=19, 20..) => self.cata_targets.1 = Some(self.get_random_defender_building().name),
            (_, 20..) => {
                self.cata_targets.0 = Some(self.get_random_defender_building().name);
                self.cata_targets.1 = Some(self.get_random_defender_building().name)
            }
            (_, _) => {
                self.cata_targets.1 = None;
            }
        };

        // FIXME: use real slot id!
        let slot_id: u8 = 1;
        for building_name in self.cata_targets.targets() {
            if let Some(b) = self.defender_village.get_building_by_name(building_name) {
                let cata_needed =
                    self.get_siege_units_needed(morale, b.level, cata_smithy, buildings_durability);

                if working_catas / self.cata_targets.targets().len() as u64 >= cata_needed {
                    // Destroy building
                    let _ = self.defender_village.destroy_building(slot_id);
                } else {
                    // Downgrade building level by a certain damage
                    let new_lvl =
                        self.calculate_building_level_damage(b.level, working_catas, cata_needed);
                    let _ = self
                        .defender_village
                        .downgrade_building_to_level(slot_id, new_lvl);
                }
            }
        }
        // TODO: buildings are damaged, but we need to include this outcome in BattleReport
    }

    // Applies damage to wall when hit by rams.
    fn apply_rams_damage(&mut self) {
        let working_rams = self.get_working_siege_units(self.attacker_army.unit_amount(6));
        if working_rams <= 0 {
            return;
        }
        let morale = self.get_siege_morale();
        let ram_smithy = self.attacker_army.smithy[6];
        let buildings_durability = self.defender_village.get_buildings_durability();
        let mut wall_level = 0;

        if let Some(wall) = self.defender_village.get_wall() {
            wall_level = wall.level;
        }

        let rams_needed =
            self.get_siege_units_needed(morale, wall_level, ram_smithy, buildings_durability);

        // FIXME: use real slot id!
        let slot_id = 1;
        if working_rams >= rams_needed {
            // Destroy building
            let _ = self.defender_village.destroy_building(slot_id);
        } else {
            // Downgrade wall level by a certain damage
            let new_lvl =
                self.calculate_building_level_damage(wall_level, working_rams, rams_needed);
            let _ = self
                .defender_village
                .downgrade_building_to_level(slot_id, new_lvl);
        }
        // TODO: wall is damaged, but we need to include this outcome in BattleReport
    }

    // Returns a random building from defender's village to be used as catapult target.
    // FIXME: should return Building name + slot_id (for applying damage to specific one)
    fn get_random_defender_building(&self) -> Building {
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..self.defender_village.buildings.len()) as u8;
        self.defender_village.buildings[&idx].clone()
    }

    // Calculates working catapults/rams based on battle points.
    fn get_working_siege_units(&self, units: u64) -> u64 {
        let battle_ratio =
            ((self.state.atk_points as f64) / (self.state.def_points as f64)).powf(1.5);

        if battle_ratio >= 1.0 {
            // Attacker is bigger/stronger
            ((units as f64) * (1.0 - 0.5 / battle_ratio)).floor() as u64
        } else {
            // Defender is bigger/stronger
            ((units as f64) * 0.5 * battle_ratio).floor() as u64
        }
    }

    // Calculates morale (based on account population of attacker and defender) for siege units.
    fn get_siege_morale(&self) -> f64 {
        // FIXME: we should use accounts populations, not villages
        let atk_pop = self.attacker_village.population;
        let def_pop = self.defender_village.population;
        // 100% ≤ morale ≤ 300%
        (atk_pop as f64 / def_pop as f64)
            .powf(0.3)
            .max(1.0)
            .min(3.0)
    }

    // Calculates amount of catapults/rams needed to destroy a building/wall.
    fn get_siege_units_needed(&self, morale: f64, level: u8, smithy: u8, durability: u16) -> u64 {
        ((morale * ((level * level) + level + 1) as f64 / (8 * smithy as u16 / durability) as f64)
            + 0.5)
            .round() as u64
    }

    // Determine the damage caused by a siege unit to a building.
    fn calculate_building_level_damage(
        &self,
        level: u8,
        working_units: u64,
        units_needed: u64,
    ) -> u8 {
        let damage_percent = (working_units * 100) as f64 / units_needed as f64;
        (level as f64 * damage_percent).floor() as u8
    }
}
