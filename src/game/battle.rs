use std::f64;

/// Battle
use crate::game::models::{
    army::{Army, UnitRole},
    village::Village,
    Tribe,
};

// Definizione delle strutture per le unità, gli eserciti e i risultati della battaglia
#[derive(Debug)]
pub struct BattleResult {
    /// True se attaccante vince.
    pub winner: bool,
    /// Perdite percentuali dell'attaccante (da 0.0 a 1.0).
    pub attacker_losses: f64,
    /// Perdite percentuali del difensore (da 0.0 a 1.0).
    pub defender_losses: f64,
    /// Danno inflitto dalle catapulte.
    pub building_damage: f64,
    /// Nuovo livello dell'edificio dopo l'attacco.
    pub new_building_level: u8,
    /// Danno inflitto dagli arieti.
    pub wall_damage: f64,
    /// Nuovo livello delle mura dopo l'attacco.
    pub new_wall_level: u8,
}

pub struct Battle;

impl Battle {
    pub fn new() -> Self {
        Battle
    }

    /// Calcola il risultato di una battaglia.
    ///
    /// # Arguments
    ///
    /// * `attacker_army` - L'esercito attaccante.
    /// * `defender_village` - Il villaggio difensore (che contiene il proprio esercito).
    /// * `reinforcements` - Un vettore opzionale di eserciti di rinforzo presenti nel villaggio.
    /// * `target_building_level` - Il livello dell'edificio bersagliato dalle catapulte (se presenti).
    pub fn calculate_battle(
        &self,
        attacker_army: &Army,
        defender_village: &Village,
        reinforcements: Option<Vec<&Army>>,
        target_building_level: u8,
    ) -> BattleResult {
        // --- Punti Attacco ---
        let (attacker_inf_atk, attacker_cav_atk) = attacker_army.attack_points();
        let total_attacker_points = attacker_inf_atk + attacker_cav_atk;

        let hero_attack_bonus = attacker_army
            .hero
            .as_ref()
            .map_or(0.0, |h| h.get_attack_bonus(true) as f64);
        let hero_off_bonus = attacker_army.hero.as_ref().map_or(0.0, |h| h.off_bonus);

        let total_attacker_with_hero_bonus = total_attacker_points as f64 + hero_attack_bonus;
        let final_attacker_points = total_attacker_with_hero_bonus as f64 * (1.0 + hero_off_bonus);

        // --- Punti Difesa ---
        let (mut total_defender_inf_def, mut total_defender_cav_def) =
            defender_village.army.defense_points();

        if let Some(hero) = &defender_village.army.hero {
            let hero_def_bonus = hero.get_defense_bonus() as f64;
            total_defender_inf_def += hero_def_bonus as u32;
            total_defender_cav_def += hero_def_bonus as u32;
        }

        if let Some(reinfs) = &reinforcements {
            for army in reinfs {
                let (inf_def, cav_def) = army.defense_points();
                total_defender_inf_def += inf_def;
                total_defender_cav_def += cav_def;
                if let Some(hero) = &army.hero {
                    let hero_def_bonus = hero.get_defense_bonus() as f64;
                    total_defender_inf_def += hero_def_bonus as u32;
                    total_defender_cav_def += hero_def_bonus as u32;
                }
            }
        }

        let total_defender_points = if total_attacker_points > 0 {
            (attacker_inf_atk / total_attacker_points) * total_defender_inf_def
                + (attacker_cav_atk / total_attacker_points) * total_defender_cav_def
        } else {
            0
        };

        // --- Danno Arieti e Livello Muro Effettivo ---
        let initial_wall_level = defender_village.get_wall().map_or(0, |b| b.level);
        let rams_quantity = attacker_army.get_troop_count_by_role(UnitRole::Ram) as f64;

        let preliminary_wall_damage =
            self.calculate_building_damage(rams_quantity, 1.0, 1.0, 1.0, 1.0, 1.0); // Danno base per ridurre il bonus
        let effective_wall_level =
            self.calculate_new_building_level(initial_wall_level, preliminary_wall_damage);

        // --- Bonus Difensivi con Muro Indebolito ---
        let residence_level = defender_village
            .get_palace_or_residence()
            .map_or(0, |(b, _)| b.level);

        let wall_factor = match defender_village.tribe {
            Tribe::Roman => 1.030f64.powi(effective_wall_level as i32),
            Tribe::Teuton => 1.020f64.powi(effective_wall_level as i32),
            Tribe::Gaul => 1.025f64.powi(effective_wall_level as i32),
            _ => 1.0,
        };
        let residence_bonus = 2.0 * (residence_level as f64).powi(2);
        let final_defender_points = (total_defender_points as f64 + residence_bonus) * wall_factor;

        // --- Morale ---
        let moral_bonus = if attacker_army.immensity() > defender_village.population {
            (defender_village.population as f64 / attacker_army.immensity() as f64)
                .powf(0.2)
                .max(0.667)
        } else {
            1.0
        };
        let effective_attack_points = final_attacker_points * moral_bonus;

        // --- Calcolo Perdite ---
        let total_units_involved = attacker_army.immensity() + defender_village.army.immensity();
        let m_factor = if total_units_involved >= 1000 {
            2.0 * (1.8592 - (total_units_involved as f64).powf(0.015))
        } else {
            1.5
        };

        let winner = effective_attack_points > final_defender_points;
        let ratio = (final_defender_points / effective_attack_points.max(1.0)).powf(m_factor);
        let (attacker_losses, defender_losses) = if winner {
            (ratio / (1.0 + ratio), 1.0)
        } else {
            (1.0, 1.0 - (ratio / (1.0 + ratio)))
        };
        let surviving_attackers_ratio = 1.0 - attacker_losses;

        // --- Danni Finali alle Costruzioni ---
        let catapults_quantity = attacker_army.get_troop_count_by_role(UnitRole::Cata) as f64
            * surviving_attackers_ratio;
        let final_rams_quantity = rams_quantity * surviving_attackers_ratio;
        let ad_ratio = effective_attack_points / final_defender_points.max(1.0);

        let building_damage = self.calculate_building_damage(
            catapults_quantity,
            1.0,
            defender_village.get_buildings_durability() as f64,
            ad_ratio,
            1.0,
            1.0,
        );
        let new_building_level =
            self.calculate_new_building_level(target_building_level, building_damage);

        let wall_damage =
            self.calculate_building_damage(final_rams_quantity, 1.0, 1.0, ad_ratio, 1.0, 1.0);
        let new_wall_level = self.calculate_new_building_level(initial_wall_level, wall_damage);

        BattleResult {
            winner,
            attacker_losses,
            defender_losses,
            building_damage,
            new_building_level,
            wall_damage,
            new_wall_level,
        }
    }

    /// Funzione di Kirilloid per calcolare l'efficienza delle catapulte.
    fn sigma(&self, x: f64) -> f64 {
        if x > 1.0 {
            (2.0 - x.powf(-1.5)) / 2.0
        } else {
            x.powf(1.5) / 2.0
        }
    }

    /// Calcola il danno inflitto da macchine d'assedio (catapulte/arieti).
    pub fn calculate_building_damage(
        &self,
        quantity: f64,
        upgrade_multiplier: f64,
        durability: f64,
        ad_ratio: f64,
        stronger_buildings_bonus: f64,
        morale_bonus: f64,
    ) -> f64 {
        if quantity <= 0.0 {
            return 0.0;
        }
        let efficiency = (quantity / (durability * stronger_buildings_bonus)).floor();
        4.0 * self.sigma(ad_ratio) * efficiency * upgrade_multiplier / morale_bonus
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
