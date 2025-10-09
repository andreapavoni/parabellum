/// Battle
use crate::game::models::{
    army::{Army, UnitRole},
    village::Village,
    Tribe,
};

// Definizione delle strutture per le unità, gli eserciti e i risultati della battaglia
#[derive(Debug)]
pub struct BattleResult {
    pub winner: bool,         // true se l'attaccante vince
    pub attacker_losses: f64, // Percentuale di perdite
    pub defender_losses: f64,
    pub building_damage: f64,
    pub new_building_level: u8,
}

pub struct Battle;

impl Battle {
    pub fn new() -> Self {
        Battle
    }

    pub fn calculate_battle(
        &self,
        attacker_army: &Army,
        defender_village: &Village,
        reinforcements: Option<Vec<&Army>>,
    ) -> BattleResult {
        // Punti di attacco
        let (attacker_inf_atk, attacker_cav_atk) = attacker_army.attack_points();
        let total_attacker_atk =
            attacker_inf_atk + attacker_cav_atk + attacker_army.hero_attack_bonus;
        let total_attacker_atk_with_bonus =
            total_attacker_atk as f64 * (1.0 + attacker_army.hero_off_bonus);

        // Punti di difesa
        let (mut total_defender_inf_def, mut total_defender_cav_def) =
            defender_village.army.defense_points();
        if let Some(reinfs) = reinforcements {
            for army in reinfs {
                let (inf_def, cav_def) = army.defense_points();
                total_defender_inf_def += inf_def;
                total_defender_cav_def += cav_def;
            }
        }
        let total_defender_def = (attacker_inf_atk / total_attacker_atk) * total_defender_inf_def
            + (attacker_cav_atk / total_attacker_atk) * total_defender_cav_def;

        // Bonus mura e residenza
        let wall_level = defender_village.get_wall().map_or(0, |b| b.level);
        let residence_level = defender_village
            .get_palace_or_residence()
            .map_or(0, |(b, _)| b.level);

        let wall_bonus = match defender_village.tribe {
            Tribe::Roman => 1.030f64.powi(wall_level as i32),
            Tribe::Teuton => 1.020f64.powi(wall_level as i32),
            Tribe::Gaul => 1.025f64.powi(wall_level as i32),
            _ => 1.0,
        };
        let residence_bonus = 2.0 * (residence_level as f64).powi(2);
        let defender_def_with_bonus = (total_defender_def as f64 + residence_bonus) * wall_bonus;

        // Morale
        let moral_bonus = if attacker_army.immensity() > defender_village.population {
            (attacker_army.immensity() as f64 / defender_village.population as f64).powf(0.3)
        } else {
            1.0
        };
        let attacker_atk_with_bonus = total_attacker_atk_with_bonus / moral_bonus;

        // Calcolo perdite
        let m_factor = if attacker_army.immensity() >= 1000 {
            2.0 * (1.8592 - (attacker_army.immensity() as f64).powf(0.015))
        } else {
            1.5
        };

        let winner = attacker_atk_with_bonus > defender_def_with_bonus;
        let (attacker_losses, defender_losses) = if winner {
            let ratio = (defender_def_with_bonus / attacker_atk_with_bonus).powf(m_factor);
            (ratio / (1.0 + ratio), 1.0 - (ratio / (1.0 + ratio)))
        } else {
            let ratio = (attacker_atk_with_bonus / defender_def_with_bonus).powf(m_factor);
            (1.0 - (ratio / (1.0 + ratio)), ratio / (1.0 + ratio))
        };

        // Danni alle costruzioni
        let catapults_quantity = attacker_army.get_troop_count_by_role(UnitRole::Cata) as f64;
        let building_damage = self.calculate_catapults_damage(
            catapults_quantity,
            1.0, // upgrade
            defender_village.get_buildings_durability() as f64,
            attacker_atk_with_bonus / defender_def_with_bonus,
            1.0, // stronger_buildings
            moral_bonus,
        );
        let new_building_level = self.calculate_new_building_level(20, building_damage); // Esempio: edificio a livello 20

        BattleResult {
            winner,
            attacker_losses,
            defender_losses,
            building_damage,
            new_building_level,
        }
    }

    fn sigma(&self, x: f64) -> f64 {
        if x > 1.0 {
            (2.0 - x.powf(-1.5)) / 2.0
        } else {
            x.powf(1.5) / 2.0
        }
    }

    pub fn calculate_catapults_damage(
        &self,
        catapults_quantity: f64,
        catapults_upgrade: f64,
        durability: f64,
        ad_ratio: f64,
        stronger_buildings: f64,
        morale_bonus: f64,
    ) -> f64 {
        let catapults_efficiency = (catapults_quantity / (durability * stronger_buildings)).floor();
        4.0 * self.sigma(ad_ratio) * catapults_efficiency * catapults_upgrade / morale_bonus
    }

    pub fn calculate_new_building_level(&self, old_level: u8, mut damage: f64) -> u8 {
        damage -= 0.5;
        if damage < 0.0 {
            return old_level;
        }

        let mut current_level = old_level;
        while damage >= current_level as f64 && current_level > 0 {
            damage -= current_level as f64;
            current_level -= 1;
        }
        current_level
    }
}
