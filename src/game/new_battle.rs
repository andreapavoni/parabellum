// Enum per definire il tipo di attacco
pub enum AttackType {
    Scout, // Esplorazione
    Raid,  // Raid/Attacco rapido
    Normal, // Attacco normale
}


// Statistiche dell'eroe rilevanti per la battaglia
pub struct HeroStats {
    pub attack_bonus_percentage: f64, // Bonus offensivo per l'esercito
    pub defense_bonus_percentage: f64, // Bonus difensivo per l'esercito
    pub base_attack: u32,
    pub base_infantry_defense: u32,
    pub base_cavalry_defense: u32,
}

// Contesto completo della battaglia
pub struct BattleContext {
    pub attack_type: AttackType,
    pub attacker: Army,
    pub defender_village_owner: Army,
    pub defender_reinforcements: Vec<Army>,

    // Stato del villaggio difensore
    pub wall_level: u8,
    pub residence_palace_level: u8,
    pub stonemason_level: u8,
    pub base_durability_artifact_multiplier: f64, // Moltiplicatore da artefatti

    // Informazioni sulle catapulte
    pub catapult_targets: Vec<BuildingTarget>, // Lista di edifici target
}

// Struttura per il risultato
pub struct BattleResult {
    pub attacker_casualties: HashMap<u8, u32>,
    pub defender_casualties: HashMap<u8, u32>,
    pub reinforcement_casualties: Vec<HashMap<u8, u32>>,
    pub wall_new_level: u8,
    pub building_damages: HashMap<BuildingTarget, u8>, // Danni agli edifici
    pub bounty_carried: u32,
    // ... altri dati come salute eroe persa, etc.
}

pub struct ScoutBattleResult {
    // Indica se la presenza di scout difensori ha "rilevato" l'attacco.
    // Se true, il difensore riceve un report dell'attacco.
    pub was_detected: bool,

    // Mappa con le perdite degli scout attaccanti.
    // Conterrà solo le unità di tipo scout.
    pub attacker_casualties: HashMap<u8, u32>,
}

pub struct BattleEngine {}

impl BattleEngine {



    // Funzione principale che orchestra tutto il calcolo
    pub fn calculate_battle(context: &BattleContext) -> BattleResult {

        // ====================================================================
        // FASE 1: Calcolo dei Punti Attacco (AP) e Difesa (DP) totali
        // ====================================================================

        let mut total_attacker_infantry_points: f64 = 0.0;
        let mut total_attacker_cavalry_points: f64 = 0.0;

        // 1.1: Calcolo punti dell'attaccante
        // Itera sulle unità dell'attaccante
        for (unit_id, count) in context.attacker.units {
            let upgrade_level = context.attacker.armory_levels.get(unit_id).unwrap_or(0);

            // `get_unit_data` è un helper che recupera statistiche base (atk, def, pop, etc.)
            let unit_data = get_unit_data(unit_id);

            let single_unit_attack = unit_data.attack +
                (unit_data.attack + 300.0 * unit_data.population / 7.0) * (1.007_f64.powi(upgrade_level) - 1.0);

            let total_unit_attack = single_unit_attack * count as f64;

            if unit_data.is_cavalry {
                total_attacker_cavalry_points += total_unit_attack;
            } else {
                total_attacker_infantry_points += total_unit_attack;
            }
        }

        // 1.2: Applica bonus Eroe attaccante
        if let Some(hero) = &context.attacker.hero {
            let hero_bonus_multiplier = 1.0 + (hero.attack_bonus_percentage / 100.0);
            total_attacker_infantry_points *= hero_bonus_multiplier;
            total_attacker_cavalry_points *= hero_bonus_multiplier;
            // Aggiungi la forza di combattimento dell'eroe (qui va interpretato se è fanteria o cavalleria)
            total_attacker_infantry_points += hero.base_attack as f64;
        }

        // 1.3: Calcolo punti del difensore (truppe proprie + rinforzi)
        let mut total_defender_infantry_points: f64 = 0.0;
        let mut total_defender_cavalry_points: f64 = 0.0;

        // Aggiungi le truppe del villaggio e tutti i rinforzi a una lista per processarli
        let mut all_defenders = vec![&context.defender_village_owner];
        all_defenders.extend(context.defender_reinforcements.iter());

        for defender_army in all_defenders {
            for (unit_id, count) in defender_army.units {
                let upgrade_level = defender_army.armory_levels.get(unit_id).unwrap_or(0);
                let unit_data = get_unit_data(unit_id);

                let single_unit_inf_def = unit_data.infantry_defense +
                    (unit_data.infantry_defense + 300.0 * unit_data.population / 7.0) * (1.007_f64.powi(upgrade_level) - 1.0);

                let single_unit_cav_def = unit_data.cavalry_defense +
                    (unit_data.cavalry_defense + 300.0 * unit_data.population / 7.0) * (1.007_f64.powi(upgrade_level) - 1.0);

                total_defender_infantry_points += single_unit_inf_def * count as f64;
                total_defender_cavalry_points += single_unit_cav_def * count as f64;
            }

            // Applica bonus Eroe difensore (se presente in questo gruppo)
            if let Some(hero) = &defender_army.hero {
                let hero_bonus_multiplier = 1.0 + (hero.defense_bonus_percentage / 100.0);
                total_defender_infantry_points *= hero_bonus_multiplier;
                total_defender_cavalry_points *= hero_bonus_multiplier;
                total_defender_infantry_points += hero.base_infantry_defense as f64;
                total_defender_cavalry_points += hero.base_cavalry_defense as f64;
            }
        }

        // 1.4: Calcolo bonus difensivi del villaggio (Mura, Residenza/Palazzo)
        // Bonus base della Residenza/Palazzo
        let residence_bonus = 2.0 * (context.residence_palace_level as f64).powi(2);
        total_defender_infantry_points += residence_bonus;
        total_defender_cavalry_points += residence_bonus;

        // Bonus percentuale delle Mura
        if context.wall_level > 0 {
            let wall_factor = match context.defender_village_owner.tribe {
                1 => 1.030, // Romani
                2 => 1.020, // Germani
                3 => 1.025, // Galli
                _ => 1.020, // Default
            };
            let wall_multiplier = wall_factor.powi(context.wall_level as i32);
            total_defender_infantry_points *= wall_multiplier;
            total_defender_cavalry_points *= wall_multiplier;
        }

        // ====================================================================
        // FASE 2: Calcolo della Potenza Totale e delle Perdite
        // ====================================================================

        // 2.1: Calcolo potenza totale d'attacco
        let total_attack_power = total_attacker_infantry_points + total_attacker_cavalry_points;
        if total_attack_power == 0.0 { /* gestione divisione per zero */ }

        // 2.2: Calcolo potenza totale di difesa (pesata sulla composizione dell'attacco)
        let infantry_ratio = total_attacker_infantry_points / total_attack_power;
        let cavalry_ratio = total_attacker_cavalry_points / total_attack_power;
        let total_defense_power = (total_defender_infantry_points * infantry_ratio) + (total_defender_cavalry_points * cavalry_ratio);

        // 2.3: Bonus Morale (se popolazione attaccante > difensore)
        let mut morale_bonus = 1.0;
        let total_defender_population = context.defender_village_owner.population +
                                       context.defender_reinforcements.iter().map(|a| a.population).sum::<u32>();

        if context.attacker.population > total_defender_population {
            let ratio = total_defender_population as f64 / context.attacker.population as f64;
            morale_bonus = ratio.powf(0.2); // Semplificato, la formula originale è più complessa
        }

        let effective_attack_power = total_attack_power / morale_bonus;

        // 2.4: Formula di combattimento
        let power_ratio = effective_attack_power / total_defense_power;

        // Fattore "Grande Battaglia" (Mfactor)
        let total_units_involved = context.attacker.units.values().sum::<u32>() +
                                  all_defenders.iter().flat_map(|a| a.units.values()).sum::<u32>();
        let m_factor = if total_units_involved >= 1000 {
            (2.0 * (1.8592 - (total_units_involved as f64).powf(0.015))).clamp(1.2578, 1.5)
        } else {
            1.5
        };

        let mut attacker_loss_percentage: f64;
        let mut defender_loss_percentage: f64;

        if effective_attack_power > total_defense_power { // Attaccante vince
            let loss_factor = (total_defense_power / effective_attack_power).powf(m_factor);

            if context.attack_type == AttackType::Raid {
                attacker_loss_percentage = loss_factor / (1.0 + loss_factor);
                defender_loss_percentage = 1.0 / (1.0 + loss_factor);
            } else { // Normal
                attacker_loss_percentage = loss_factor;
                defender_loss_percentage = 1.0;
            }
        } else { // Difensore vince (o pareggio)
            let loss_factor = (effective_attack_power / total_defense_power).powf(m_factor);

            if context.attack_type == AttackType::Raid {
                attacker_loss_percentage = 1.0 / (1.0 + loss_factor);
                defender_loss_percentage = loss_factor / (1.0 + loss_factor);
            } else { // Normal
                attacker_loss_percentage = 1.0;
                defender_loss_percentage = loss_factor;
            }
        }

        // ====================================================================
        // FASE 3: Calcolo Danni a Mura ed Edifici
        // ====================================================================

        let mut final_wall_level = context.wall_level;
        // ... altre variabili per danni agli edifici

        // 3.1: Danno Arieti (Rams)
        let surviving_rams = calculate_surviving_units(&context.attacker, "rams", attacker_loss_percentage);
        if surviving_rams > 0 && context.wall_level > 0 {
            let ram_damage = calculate_machine_damage(
                surviving_rams,
                context.attacker.armory_levels.get("ram_id").unwrap_or(0), // Upgrade arieti
                context.stonemason_level,
                context.base_durability_artifact_multiplier,
                power_ratio,
                1.0 // Morale per arieti è 1.0
            );
            final_wall_level = calculate_new_building_level(context.wall_level, ram_damage);
        }

        // 3.2: Danno Catapulte
        let surviving_catapults = calculate_surviving_units(&context.attacker, "catapults", attacker_loss_percentage);
        if surviving_catapults > 0 {
            let catapult_morale_bonus = (context.attacker.population as f64 / total_defender_population as f64).powf(0.3).clamp(1.0, 3.0);

            // ... logica per distribuire le catapulte sui bersagli ...
            let catapult_damage = calculate_machine_damage(
                surviving_catapults, // Quantità per questo target
                context.attacker.armory_levels.get("catapult_id").unwrap_or(0), // Upgrade catapulte
                context.stonemason_level,
                context.base_durability_artifact_multiplier,
                power_ratio,
                catapult_morale_bonus
            );
            // ... calcola il nuovo livello per l'edificio target
        }

        // ====================================================================
        // FASE 4: Finalizzazione dei Risultati
        // ====================================================================

        // Calcola le perdite effettive per ogni tipo di unità e assembla la struttura BattleResult
        // ...

        // Restituisci il risultato completo
        return BattleResult { ... };
    }

    pub fn calculate_scout_battle(context: &BattleContext) -> ScoutBattleResult {

        // ====================================================================
        // FASE 1: Calcolo Punti Attacco e Difesa degli Scout
        // ====================================================================

        let mut total_scout_attack_power: f64 = 0.0;
        let mut total_attacker_scouts: u32 = 0;

        // 1.1: Calcola la forza degli scout attaccanti
        for (unit_id, count) in &context.attacker.units {
            let unit_data = get_unit_data(*unit_id);
            if unit_data.is_scout {
                total_attacker_scouts += *count;
                let upgrade_level = context.attacker.armory_levels.get(unit_id).unwrap_or(0);

                // Formula specifica per scout: base 35
                let single_unit_power = 35.0 +
                    (35.0 + 300.0 * unit_data.population / 7.0) * (1.007_f64.powi(upgrade_level) - 1.0);

                total_scout_attack_power += single_unit_power * (*count as f64);
            }
        }

        let mut total_scout_defense_power: f64 = 0.0;
        let mut defender_has_scouts = false;
        let mut total_defender_scouts: u32 = 0;

        // 1.2: Calcola la forza degli scout difensori (truppe proprie + rinforzi)
        let all_defenders = [&context.defender_village_owner]
            .into_iter()
            .chain(context.defender_reinforcements.iter());

        for defender_army in all_defenders {
            for (unit_id, count) in &defender_army.units {
                let unit_data = get_unit_data(*unit_id);
                if unit_data.is_scout && *count > 0 {
                    defender_has_scouts = true;
                    total_defender_scouts += *count;
                    let upgrade_level = defender_army.armory_levels.get(unit_id).unwrap_or(0);

                    // Formula specifica per scout: base 20
                    let single_unit_power = 20.0 +
                        (20.0 + 300.0 * unit_data.population / 7.0) * (1.007_f64.powi(upgrade_level) - 1.0);

                    total_scout_defense_power += single_unit_power * (*count as f64);
                }
            }
        }

        // ====================================================================
        // FASE 2: Applica Bonus e Calcola Perdite
        // ====================================================================

        // 2.1: Applica il bonus delle mura (se ci sono difensori)
        if defender_has_scouts && context.wall_level > 0 {
            let wall_factor = match context.defender_village_owner.tribe {
                1 => 1.030, // Romani
                2 => 1.020, // Germani
                3 => 1.025, // Galli
                _ => 1.020,
            };
            let wall_multiplier = wall_factor.powi(context.wall_level as i32);
            total_scout_defense_power *= wall_multiplier;
            total_scout_defense_power += 10.0; // Bonus fisso
        }

        let mut attacker_loss_percentage = 0.0;

        // 2.2: Controlla le condizioni per le perdite
        // La tribù 5 sono gli Egiziani nel codice originale di TravianZ
        let attacker_is_immune = context.attacker.tribe == 5;

        if !attacker_is_immune && defender_has_scouts && total_scout_attack_power > 0.0 {
            // Se ci sono difensori e l'attaccante non è immune, calcola le perdite
            let total_units_involved = total_attacker_scouts + total_defender_scouts;
            let m_factor = if total_units_involved >= 1000 {
                (2.0 * (1.8592 - (total_units_involved as f64).powf(0.015))).clamp(1.2578, 1.5)
            } else {
                1.5
            };

            // Il morale si applica anche qui, anche se nel codice PHP è un po' nascosto
            // Omettiamolo per semplicità, ma andrebbe calcolato come prima
            let power_ratio = total_scout_defense_power / total_scout_attack_power;

            // La formula è `min(1, (difesa/attacco)^M)`.
            attacker_loss_percentage = power_ratio.powf(m_factor).min(1.0);
        }

        // ====================================================================
        // FASE 3: Finalizzazione del Risultato
        // ====================================================================

        let mut final_casualties = HashMap::new();

        // Calcola le perdite effettive per le unità scout attaccanti
        for (unit_id, count) in &context.attacker.units {
             if get_unit_data(*unit_id).is_scout {
                 let losses = (*count as f64 * attacker_loss_percentage).round() as u32;
                 final_casualties.insert(*unit_id, losses);
             }
        }

        return ScoutBattleResult {
            was_detected: defender_has_scouts,
            attacker_casualties: final_casualties,
        };
    }

    // Funzione sigma per il calcolo del danno delle catapulte
    // $this->sigma = function($x) { return ($x > 1 ? 2 - $x ** -1.5 : $x ** 1.5) / 2; };
    fn sigma(x: f64) -> f64 {
        if x > 1.0 {
            (2.0 - x.powf(-1.5)) / 2.0
        } else {
            x.powf(1.5) / 2.0
        }
    }

    // Helper per calcolare il danno di catapulte/arieti
    fn calculate_machine_damage(quantity: u32, upgrade_level: u8, stonemason: u8, artifact_mult: f64, ad_ratio: f64, morale: f64) -> f64 {
        let upgrades = 1.0 + (upgrade_level as f64 * 0.0205); // Formula semplificata, l'originale usa pow(1.0205, level)
        let durability = (1.0 + stonemason as f64 * 0.1) * artifact_mult;

        let efficiency = (quantity as f64 / durability).floor();

        return 4.0 * Self::sigma(ad_ratio) * efficiency * upgrades / morale;
    }

    // Helper per calcolare il nuovo livello di un edificio
    fn calculate_new_building_level(old_level: u8, mut damage: f64) -> u8 {
        let mut current_level = old_level;
        damage -= 0.5;
        if damage < 0.0 { return current_level; }

        while damage >= current_level as f64 && current_level > 0 {
            damage -= current_level as f64;
            current_level -= 1;
        }
        return current_level;
    }


}
