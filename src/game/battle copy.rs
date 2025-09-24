use std::{collections::HashMap, f64};

use serde::{Deserialize, Serialize};

/// Battle
use crate::game::models::{
    army::{Army, UnitRole},
    village::Village,
    Tribe,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BattleType {
    Normal,
    Raid,
    Scout,
}

// Battle input data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleInput {
    pub battle_type: BattleType,
    pub attacker: Army,
    pub defender_village: Village,
    /// Catapults targets
    pub cata_target_slot_id_1: Option<u8>,
    pub cata_target_slot_id_2: Option<u8>,
}

impl BattleInput {
    pub fn new(
        battle_type: BattleType,
        attacker: Army,
        defender_village: Village,
        cata_target_slot_id_1: Option<u8>,
        cata_target_slot_id_2: Option<u8>,
    ) -> Self {
        Self {
            battle_type,
            attacker,
            defender_village,
            cata_target_slot_id_1,
            cata_target_slot_id_2,
        }
    }
}

/// Battle result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleReport {
    pub attacker_losses: HashMap<u32, u32>,
    pub defender_losses: HashMap<u32, u32>, // Qui aggregheremo le perdite di tutti i difensori
    pub new_wall_level: u8,
    /// Bounty is then divided by the four resources
    pub bounty: u32,
    pub target_new_level_1: HashMap<u8, u8>,
    pub target_new_level_2: HashMap<u8, u8>,
}

impl BattleReport {
    pub fn new(
        attacker_losses: HashMap<u32, u32>,
        defender_losses: HashMap<u32, u32>,
        new_wall_level: u8,
        bounty: u32,
        target_new_level_1: HashMap<u8, u8>,
        target_new_level_2: HashMap<u8, u8>,
    ) -> Self {
        Self {
            attacker_losses,
            defender_losses,
            new_wall_level,
            bounty,
            target_new_level_1,
            target_new_level_2,
        }
    }
}

/// Main function to calculate a battle.
pub fn calculate_battle(input: BattleInput) -> BattleReport {
    match input.battle_type {
        BattleType::Scout => calculate_scout_battle(&input),
        _ => calculate_combat_battle(&input),
    }
}

/// Calculates scount battle.
fn calculate_scout_battle(input: &BattleInput) -> BattleReport {
    // PLACEHOLDER: La logica per gli scout è diversa.
    // 1. Calcola la "forza scout" dell'attaccante.
    let atk_points = input.attacker.scouting_attack_points();
    let mut def_points = input.defender_village.army.scouting_defense_points();

    // 2. Calcola la "forza difensiva scout" del difensore.
    let reinforcements_scout_points: u32 = input
        .defender_village
        .reinforcements
        .iter()
        .map(|r| r.scouting_defense_points())
        .sum();

    def_points = def_points + reinforcements_scout_points;

    // 3. Applica una formula simile ma più semplice per determinare le perdite.
    // 4. Se la forza attaccante è maggiore, il report non viene generato per il difensore.
    unimplemented!();
}

fn calculate_combat_battle(input: &BattleInput) -> BattleReport {
    // --- FASE 1: CALCOLO PUNTI BASE ---
    let (mut inf_attack_points, mut cav_attack_points) = input.attacker.attack_points();

    // Applica bonus eroe all'attacco
    if let Some(hero) = input.attacker.hero {
        hero.attack_points
    }
    let hero_off_bonus_multiplier = 1.0 + (input.attacker.hero_attack_bonus_percent / 100.0);
    inf_attack_points *= hero_off_bonus_multiplier;
    cav_attack_points *= hero_off_bonus_multiplier;
    inf_attack_points += input.attacker.hero_fighting_strength as f64; // Forza base eroe si somma alla fanteria

    let (mut inf_defense_points, mut cav_defense_points) = (0.0, 0.0);
    for army in &input.reinforcements {
        let (def_inf, def_cav) = calculate_base_defense_points(army);
        inf_defense_points += def_inf;
        cav_defense_points += def_cav;
    }

    // --- FASE 2: CALCOLO PUNTI EFFETTIVI E BONUS ---
    let total_attack_points = inf_attack_points + cav_attack_points;
    if total_attack_points < 1.0 {
        /* Evita divisione per zero */
        return generate_empty_report(input);
    }

    // Proporzione fanteria/cavalleria dell'attaccante
    let inf_proportion = inf_attack_points / total_attack_points;
    let cav_proportion = cav_attack_points / total_attack_points;

    // Calcolo difesa pesata
    let mut total_defense_points =
        (inf_defense_points * inf_proportion) + (cav_defense_points * cav_proportion);

    // Applica bonus Muro e Residenza
    let wall_bonus = calculate_wall_bonus(input.reinforcements[0].tribe, input.wall_level);
    total_defense_points *= wall_bonus;
    total_defense_points += (2.0 * (input.residence_level as f64).powi(2)) * wall_bonus;

    // Applica bonus Morale
    let morale_bonus = calculate_morale_bonus(
        input.attacker.population,
        input.reinforcements.iter().map(|a| a.population).sum(),
    );
    total_defense_points *= morale_bonus;

    if total_defense_points < 1.0 {
        total_defense_points = 1.0;
    }

    // --- FASE 3: FORMULA DI COMBATTIMENTO E PERDITE ---
    let total_troops_involved = input.attacker.units.values().sum::<u32>()
        + input
            .reinforcements
            .iter()
            .flat_map(|a| a.units.values())
            .sum::<u32>();
    let m_factor = calculate_m_factor(total_troops_involved);

    let power_ratio = total_attack_points / total_defense_points;

    let (attacker_loss_percent, defender_loss_percent) = if power_ratio > 1.0 {
        // Attaccante vince
        let defender_loss = 1.0; // 100%
        let attacker_loss = (1.0 / power_ratio).powf(m_factor);
        (attacker_loss, defender_loss)
    } else {
        // Difensore vince (o pareggio)
        let attacker_loss = 1.0; // 100%
        let defender_loss = power_ratio.powf(m_factor);
        (attacker_loss, defender_loss)
    };

    // Logica specifica per il tipo di attacco (Raid ha perdite ridotte)
    let (final_attacker_loss, final_defender_loss) = match input.battle_type {
        BattleType::Raid => {
            let holder = power_ratio.powf(m_factor);
            let raid_loss = holder / (1.0 + holder);
            if power_ratio > 1.0 {
                // Attaccante vince
                (raid_loss, 1.0 - raid_loss)
            } else {
                // Difensore vince
                (1.0 - raid_loss, raid_loss)
            }
        }
        _ => (attacker_loss_percent, defender_loss_percent),
    };

    // --- FASE 4: POST-BATTAGLIA (ARIETI E CATAPULTE) ---
    // (Questa sezione è semplificata, la logica completa è molto complessa)

    // Calcola sopravvissuti per calcolo demolizione
    let surviving_rams = 0; // PLACEHOLDER: Calcola arieti sopravvissuti
    let surviving_catapults = 0; // PLACEHOLDER: Calcola catapulte sopravvissute

    let new_wall_level = if surviving_rams > 0 {
        // La formula `calculate_demolition_damage` nel PHP è complessa
        // e usa una funzione `sigma`. La replico qui.
        let sigma = |x: f64| {
            if x > 1.0 {
                (2.0 - x.powf(-1.5)) / 2.0
            } else {
                x.powf(1.5) / 2.0
            }
        };
        // ... Logica per calcolare il danno degli arieti
        input.wall_level // placeholder
    } else {
        input.wall_level
    };

    // ... Logica simile per le catapulte e il target_building ...

    // --- FASE 5: GENERAZIONE REPORT ---
    // Calcola le perdite effettive e il bottino

    let mut attacker_losses = HashMap::new();
    for (id, quantity) in &input.attacker.units {
        let lost = ((*quantity as f64) * final_attacker_loss).round() as u32;
        attacker_losses.insert(*id, lost);
    }

    // Qui semplifico aggregando tutte le perdite dei difensori in una mappa
    let mut defender_losses = HashMap::new();
    // ... Logica per calcolare e aggregare le perdite per tutti i difensori ...

    // ... Logica per calcolare il bottino ...

    BattleReport {
        attacker_losses,
        defender_losses,
        new_wall_level,
        new_target_building_level: input.target_building_level, // Placeholder
        bounty: 0,                                              // Placeholder
    }
}

// pub struct Battle;

// impl Battle {
//     pub fn new() -> Self {
//         Battle
//     }

//     /// Calcola il risultato di una battaglia.
//     ///
//     /// # Arguments
//     ///
//     /// * `attacker_army` - L'esercito attaccante.
//     /// * `defender_village` - Il villaggio difensore (che contiene il proprio esercito).
//     /// * `reinforcements` - Un vettore opzionale di eserciti di rinforzo presenti nel villaggio.
//     /// * `target_building_level` - Il livello dell'edificio bersagliato dalle catapulte (se presenti).
//     pub fn calculate_battle(
//         &self,
//         attacker_army: &Army,
//         defender_village: &Village,
//         reinforcements: Option<Vec<&Army>>,
//         target_building_level: u8,
//     ) -> BattleResult {
//         // --- Punti Attacco ---
//         let (attacker_inf_atk, attacker_cav_atk) = attacker_army.attack_points();
//         let total_attacker_points = attacker_inf_atk + attacker_cav_atk;

//         let hero_attack_bonus = attacker_army
//             .hero
//             .as_ref()
//             .map_or(0.0, |h| h.get_attack_bonus(true) as f64);
//         let hero_off_bonus = attacker_army.hero.as_ref().map_or(0.0, |h| h.off_bonus);

//         let total_attacker_with_hero_bonus = total_attacker_points as f64 + hero_attack_bonus;
//         let final_attacker_points = total_attacker_with_hero_bonus as f64 * (1.0 + hero_off_bonus);

//         // --- Punti Difesa ---
//         let (mut total_defender_inf_def, mut total_defender_cav_def) =
//             defender_village.army.defense_points();

//         if let Some(hero) = &defender_village.army.hero {
//             let hero_def_bonus = hero.get_defense_bonus() as f64;
//             total_defender_inf_def += hero_def_bonus as u32;
//             total_defender_cav_def += hero_def_bonus as u32;
//         }

//         if let Some(reinfs) = &reinforcements {
//             for army in reinfs {
//                 let (inf_def, cav_def) = army.defense_points();
//                 total_defender_inf_def += inf_def;
//                 total_defender_cav_def += cav_def;
//                 if let Some(hero) = &army.hero {
//                     let hero_def_bonus = hero.get_defense_bonus() as f64;
//                     total_defender_inf_def += hero_def_bonus as u32;
//                     total_defender_cav_def += hero_def_bonus as u32;
//                 }
//             }
//         }

//         let total_defender_points = if total_attacker_points > 0 {
//             (attacker_inf_atk / total_attacker_points) * total_defender_inf_def
//                 + (attacker_cav_atk / total_attacker_points) * total_defender_cav_def
//         } else {
//             0
//         };

//         // --- Danno Arieti e Livello Muro Effettivo ---
//         let initial_wall_level = defender_village.get_wall().map_or(0, |b| b.level);
//         let rams_quantity = attacker_army.get_troop_count_by_role(UnitRole::Ram) as f64;

//         let preliminary_wall_damage =
//             self.calculate_building_damage(rams_quantity, 1.0, 1.0, 1.0, 1.0, 1.0); // Danno base per ridurre il bonus
//         let effective_wall_level =
//             self.calculate_new_building_level(initial_wall_level, preliminary_wall_damage);

//         // --- Bonus Difensivi con Muro Indebolito ---
//         let residence_level = defender_village
//             .get_palace_or_residence()
//             .map_or(0, |(b, _)| b.level);

//         let wall_factor = match defender_village.tribe {
//             Tribe::Roman => 1.030f64.powi(effective_wall_level as i32),
//             Tribe::Teuton => 1.020f64.powi(effective_wall_level as i32),
//             Tribe::Gaul => 1.025f64.powi(effective_wall_level as i32),
//             _ => 1.0,
//         };
//         let residence_bonus = 2.0 * (residence_level as f64).powi(2);
//         let final_defender_points = (total_defender_points as f64 + residence_bonus) * wall_factor;

//         // --- Morale ---
//         let moral_bonus = if attacker_army.immensity() > defender_village.population {
//             (defender_village.population as f64 / attacker_army.immensity() as f64)
//                 .powf(0.2)
//                 .max(0.667)
//         } else {
//             1.0
//         };
//         let effective_attack_points = final_attacker_points * moral_bonus;

//         // --- Calcolo Perdite ---
//         let total_units_involved = attacker_army.immensity() + defender_village.army.immensity();
//         let m_factor = if total_units_involved >= 1000 {
//             2.0 * (1.8592 - (total_units_involved as f64).powf(0.015))
//         } else {
//             1.5
//         };

//         let winner = effective_attack_points > final_defender_points;
//         let ratio = (final_defender_points / effective_attack_points.max(1.0)).powf(m_factor);
//         let (attacker_losses, defender_losses) = if winner {
//             (ratio / (1.0 + ratio), 1.0)
//         } else {
//             (1.0, 1.0 - (ratio / (1.0 + ratio)))
//         };
//         let surviving_attackers_ratio = 1.0 - attacker_losses;

//         // --- Danni Finali alle Costruzioni ---
//         let catapults_quantity = attacker_army.get_troop_count_by_role(UnitRole::Cata) as f64
//             * surviving_attackers_ratio;
//         let final_rams_quantity = rams_quantity * surviving_attackers_ratio;
//         let ad_ratio = effective_attack_points / final_defender_points.max(1.0);

//         let building_damage = self.calculate_building_damage(
//             catapults_quantity,
//             1.0,
//             defender_village.get_buildings_durability() as f64,
//             ad_ratio,
//             1.0,
//             1.0,
//         );
//         let new_building_level =
//             self.calculate_new_building_level(target_building_level, building_damage);

//         let wall_damage =
//             self.calculate_building_damage(final_rams_quantity, 1.0, 1.0, ad_ratio, 1.0, 1.0);
//         let new_wall_level = self.calculate_new_building_level(initial_wall_level, wall_damage);

//         BattleResult {
//             winner,
//             attacker_losses,
//             defender_losses,
//             building_damage,
//             new_building_level,
//             wall_damage,
//             new_wall_level,
//         }
//     }

//     /// Funzione di Kirilloid per calcolare l'efficienza delle catapulte.
//     fn sigma(&self, x: f64) -> f64 {
//         if x > 1.0 {
//             (2.0 - x.powf(-1.5)) / 2.0
//         } else {
//             x.powf(1.5) / 2.0
//         }
//     }

//     /// Calcola il danno inflitto da macchine d'assedio (catapulte/arieti).
//     pub fn calculate_building_damage(
//         &self,
//         quantity: f64,
//         upgrade_multiplier: f64,
//         durability: f64,
//         ad_ratio: f64,
//         stronger_buildings_bonus: f64,
//         morale_bonus: f64,
//     ) -> f64 {
//         if quantity <= 0.0 {
//             return 0.0;
//         }
//         let efficiency = (quantity / (durability * stronger_buildings_bonus)).floor();
//         4.0 * self.sigma(ad_ratio) * efficiency * upgrade_multiplier / morale_bonus
//     }

//     pub fn calculate_new_building_level(&self, old_level: u8, mut damage: f64) -> u8 {
//         damage -= 0.5;
//         if damage < 0.0 {
//             return old_level;
//         }
//         let mut current_level = old_level;
//         while damage >= current_level as f64 && current_level > 0 {
//             damage -= current_level as f64;
//             current_level -= 1;
//         }
//         current_level
//     }
// }

// #[cfg(test)]
// mod tests {
//     use crate::game::models::hero::Hero;

//     use super::Battle;
//     use crate::game::models::{
//         army::Army,
//         common::{Player, Tribe},
//         map::{Position, Valley, ValleyTopology},
//         village::Village,
//     };
//     use uuid::Uuid;

//     fn create_test_village(player_id: Uuid, tribe: Tribe, population: u32) -> Village {
//         let position = Position { x: 0, y: 0 };
//         let valley = Valley {
//             id: 0,
//             position,
//             topology: ValleyTopology(4, 4, 4, 6),
//             player_id: Some(player_id),
//             village_id: Some(0),
//         };
//         let player = Player {
//             id: player_id,
//             username: "test".to_string(),
//             tribe,
//         };
//         let mut village = Village::new("Test Village".to_string(), &valley, &player, false);
//         village.population = population;
//         village
//     }

//     // Funzione helper per l'uguaglianza approssimativa
//     fn assert_almost_equal(a: f64, b: f64, epsilon: f64) {
//         assert!(
//             (a - b).abs() < epsilon,
//             "{} is not almost equal to {}",
//             a,
//             b
//         );
//     }

//     #[test]
//     fn test_infantry_battle_attacker_wins() {
//         let player_id = Uuid::new_v4();
//         let attacker_army = Army::new(
//             0,
//             player_id,
//             Tribe::Roman,
//             [100, 0, 0, 0, 0, 0, 0, 0, 0, 0],
//             [0; 10],
//             None,
//         );
//         let mut defender_village = create_test_village(Uuid::new_v4(), Tribe::Teuton, 50);
//         defender_village.army = Army::new(
//             1,
//             player_id,
//             Tribe::Teuton,
//             [50, 0, 0, 0, 0, 0, 0, 0, 0, 0],
//             [0; 10],
//             None,
//         );

//         let simulator = Battle::new();
//         let result = simulator.calculate_battle(&attacker_army, &defender_village, None, 0);

//         assert!(result.winner);
//         assert!(result.attacker_losses > 0.0 && result.attacker_losses < 1.0);
//         assert_eq!(result.defender_losses, 1.0);
//     }

//     #[test]
//     fn test_infantry_battle_defender_wins() {
//         let player_id = Uuid::new_v4();
//         let attacker_army = Army::new(
//             0,
//             player_id,
//             Tribe::Roman,
//             [50, 0, 0, 0, 0, 0, 0, 0, 0, 0],
//             [0; 10],
//             None,
//         );
//         let mut defender_village = create_test_village(Uuid::new_v4(), Tribe::Teuton, 100);
//         defender_village.army = Army::new(
//             1,
//             Uuid::new_v4(),
//             Tribe::Teuton,
//             [100, 0, 0, 0, 0, 0, 0, 0, 0, 0],
//             [0; 10],
//             None,
//         );

//         let simulator = Battle::new();
//         let result = simulator.calculate_battle(&attacker_army, &defender_village, None, 0);

//         assert!(!result.winner);
//         assert_eq!(result.attacker_losses, 1.0);
//         assert!(result.defender_losses > 0.0 && result.defender_losses < 1.0);
//     }

//     #[test]
//     fn test_rams_reduce_wall_effectiveness() {
//         let player_id = Uuid::new_v4();
//         // Attaccante con molti arieti ma poca fanteria
//         let attacker_army = Army::new(
//             0,
//             player_id,
//             Tribe::Roman,
//             [10, 0, 0, 0, 0, 0, 100, 0, 0, 0],
//             [0; 10],
//             None,
//         );

//         let mut defender_village = create_test_village(Uuid::new_v4(), Tribe::Roman, 200);
//         defender_village.army = Army::new(
//             1,
//             Uuid::new_v4(),
//             Tribe::Roman,
//             [500, 0, 0, 0, 0, 0, 0, 0, 0, 0],
//             [0; 10],
//             None,
//         );
//         // Aggiungi un muro di livello 20
//         // ... (dovresti avere un metodo per aggiungere/aggiornare edifici nel villaggio per i test)
//         // Per ora, possiamo solo verificare che il danno al muro sia calcolato.

//         let simulator = Battle::new();
//         let result = simulator.calculate_battle(&attacker_army, &defender_village, None, 0);

//         // Anche se l'attaccante probabilmente perde, ci aspettiamo un danno significativo al muro.
//         assert!(!result.winner);
//         assert!(result.wall_damage > 0.0);
//         assert!(result.new_wall_level < 20); // Assumendo un muro iniziale di livello 20
//     }

//     #[test]
//     fn test_catapults_damage_building() {
//         let player_id = Uuid::new_v4();
//         // Attaccante con catapulte
//         let attacker_army = Army::new(
//             0,
//             player_id,
//             Tribe::Roman,
//             [1000, 0, 0, 0, 0, 0, 0, 50, 0, 0],
//             [20; 10],
//             None,
//         );
//         let defender_village = create_test_village(Uuid::new_v4(), Tribe::Gaul, 300);

//         let simulator = Battle::new();
//         let result = simulator.calculate_battle(&attacker_army, &defender_village, None, 15); // Bersaglia un edificio di livello 15

//         assert!(result.winner);
//         assert!(result.building_damage > 0.0);
//         assert!(result.new_building_level < 15);
//     }

//     #[test]
//     fn test_hero_is_attacking() {
//         let attacker_player_id = Uuid::new_v4();
//         let defender_player_id = Uuid::new_v4();

//         let hero = Hero {
//             player_id: attacker_player_id,
//             attack_points: 20, // Simula 'self: 20' e 'str: 1000'
//             off_bonus: 0.0,    // Assumiamo che il bonus off sia separato
//             ..Default::default()
//         };

//         let attacker_army = Army::new(
//             0,
//             attacker_player_id,
//             Tribe::Roman,
//             [0; 10], // Nessuna truppa, solo eroe
//             [0; 10],
//             Some(hero),
//         );

//         let mut defender_village = create_test_village(defender_player_id, Tribe::Gaul, 100);
//         defender_village.army = Army::new(
//             1,
//             defender_player_id,
//             Tribe::Gaul,
//             [100, 0, 0, 0, 0, 0, 0, 0, 0, 0], // 100 Falangi
//             [0; 10],
//             None,
//         );

//         let simulator = Battle::new();
//         let result = simulator.calculate_battle(&attacker_army, &defender_village, None, 0);

//         // Valore fittizio, da verificare
//         // In questo scenario, l'eroe da solo potrebbe non vincere, ma infliggerà perdite.
//         // Il valore esatto di `defLosses` dipende da come calcoli il potere dell'eroe.
//         // L'originale era (3100 / 4010.21) ** 1.5, che è ~0.67
//         assert_almost_equal(result.defender_losses, 0.67, 0.01);
//     }

//     #[test]
//     fn test_large_scale_battle_minor_change() {
//         let attacker_player_id = Uuid::new_v4();
//         let defender_player_id = Uuid::new_v4();

//         let attacker_army = Army::new(
//             0,
//             attacker_player_id,
//             Tribe::Roman,
//             [499999, 0, 0, 0, 0, 0, 0, 0, 0, 0], // 499,999 Legionari
//             [0; 10],
//             None,
//         );

//         let mut defender_village = create_test_village(defender_player_id, Tribe::Teuton, 5000);
//         defender_village.army = Army::new(
//             1,
//             defender_player_id,
//             Tribe::Teuton,
//             [999999, 0, 0, 0, 0, 0, 0, 0, 0, 0], // 999,999 Mazze
//             [0; 10],
//             None,
//         );

//         let simulator = Battle::new();
//         let result = simulator.calculate_battle(&attacker_army, &defender_village, None, 0);

//         // Valori fittizi basati sul test TS.
//         // `offLosses` dovrebbe essere 1.0 (sconfitta totale)
//         // `defLosses` molto alte ma non totali.
//         assert_eq!(result.attacker_losses, 1.0);
//         let expected_defender_survivors = 68; // 999999 - 999931
//         let actual_defender_survivors = (999999.0 * (1.0 - result.defender_losses)).round() as u32;
//         assert_eq!(actual_defender_survivors, expected_defender_survivors);
//     }
// }
