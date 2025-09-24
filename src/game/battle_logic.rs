fn calculate_battle(
    attack_army: &Army,
    defense_armies: &Vec<Army>, // include esercito principale + rinforzi
    wall_level: f32,
    wall_tribe: Tribe, // Romani/Galli/Teutoni (bonus differente)
    residence_level: f32,
    defender_population: f32,
    attack_mode: AttackMode, // Normal | Raid | Scout
    target_buildings: Option<(BuildingId, Option<BuildingId>)>,
) -> BattleResult {
    // --- 1. Calcolo attacco ---
    let (attack_infantry, attack_cavalry) = calculate_attack_points(attack_army);
    let hero_attack = attack_army.hero.map(|h| h.attack).unwrap_or(0.0);

    let total_attack_points = attack_infantry + attack_cavalry + hero_attack;

    // --- 2. Calcolo difesa ---
    let mut defense_infantry: f32 = 0.0;
    let mut defense_cavalry: f32 = 0.0;

    for army in defense_armies {
        let (di, dc) = calculate_defense_points(army);
        defense_infantry += di;
        defense_cavalry += dc;

        if let Some(hero) = &army.hero {
            defense_infantry += hero.defense_infantry;
            defense_cavalry += hero.defense_cavalry;
        }
    }

    // bonus residenza/castello
    let residence_bonus = 1.0 + (residence_level / 10.0);
    defense_infantry *= residence_bonus;
    defense_cavalry *= residence_bonus;

    // bonus muro (dipende dalla tribù)
    let wall_bonus = match wall_tribe {
        Tribe::Romans => 1.0 + (wall_level * 0.05),
        Tribe::Teutons => 1.0 + (wall_level * 0.025),
        Tribe::Gauls => 1.0 + (wall_level * 0.03),
    };
    defense_infantry *= wall_bonus;
    defense_cavalry *= wall_bonus;

    let total_defense_points = defense_infantry + defense_cavalry;

    // --- 3. Distribuzione attacco su fanteria/cavalleria ---
    let effective_defense: f32 = if total_defense_points > 0.0 {
        (attack_infantry / total_attack_points) * defense_infantry
            + (attack_cavalry / total_attack_points) * defense_cavalry
    } else {
        0.0
    };

    // --- 4. Morale bonus ---
    let morale_bonus =
        if attack_army.population as f32 > defender_population && defender_population > 0.0 {
            (defender_population / attack_army.population as f32).powf(0.2)
        } else {
            1.0
        };

    // --- 5. Rapporto di forza ---
    let attack_value = total_attack_points * morale_bonus;
    let defense_value = effective_defense;
    let ratio = attack_value / defense_value;

    // --- 6. Perdite base ---
    let sigma_value = sigma(ratio);
    let m_factor = 1.5; // costante del gioco

    let attacker_loss_ratio = if ratio > 1.0 {
        sigma_value / m_factor
    } else {
        sigma_value * m_factor
    };

    let defender_loss_ratio = if ratio > 1.0 {
        sigma_value * m_factor
    } else {
        sigma_value / m_factor
    };

    // --- 7. Tipo di attacco ---
    match attack_mode {
        AttackMode::Raid => {
            // perdite attaccante dimezzate
            attacker_loss_ratio *= 0.5;
            // difensore subisce di meno
            defender_loss_ratio *= 0.5;
        }
        AttackMode::Scout => {
            // solo esploratori partecipano, logica ridotta
            return calculate_scouting(attack_army, defense_armies);
        }
        AttackMode::Normal => {
            // nessuna modifica
        }
    }

    // --- 8. Arieti (ram) contro muro ---
    let surviving_rams = surviving_units(attack_army, UnitType::Ram, attacker_loss_ratio);
    if surviving_rams > 0 {
        let wall_damage =
            calculate_ram_damage(surviving_rams, attack_value, defense_value, wall_level);
        // riduzione effettiva livello muro
    }

    // --- 9. Catapulte contro edifici ---
    let surviving_catas = surviving_units(attack_army, UnitType::Catapult, attacker_loss_ratio);
    if surviving_catas > 0 {
        if let Some((primary, secondary)) = target_buildings {
            let catapult_damage = calculate_catapult_damage(
                surviving_catas,
                attack_value,
                defense_value,
                primary,
                secondary,
            );
            // applica danno agli edifici target
        }
    }

    // --- 10. Vincitore ---
    let winner = if ratio > 1.0 {
        BattleSide::Attacker
    } else {
        BattleSide::Defender
    };

    // --- 11. Risultato finale ---
    BattleResult {
        attack_value,
        defense_value,
        attacker_loss_ratio,
        defender_loss_ratio,
        winner,
    }
}

fn calculate_attack_points(army: &Army) -> (f32, f32) {
    let mut infantry_points = 0.0;
    let mut cavalry_points = 0.0;

    for stack in &army.units {
        let unit = UNIT[stack.unit_id];
        let upgrade_bonus = get_upgrade_bonus(stack.upgrade);
        let value = (unit.attack_points as f32) * upgrade_bonus * (stack.count as f32);

        if unit.is_cavalry {
            cavalry_points += value;
        } else {
            infantry_points += value;
        }
    }
    (infantry_points, cavalry_points)
}

fn calculate_defense_points(army: &Army) -> (f32, f32) {
    let mut di = 0.0;
    let mut dc = 0.0;

    for stack in &army.units {
        let unit = UNIT[stack.unit_id];
        let upgrade_bonus = get_upgrade_bonus(stack.upgrade);
        di += (unit.defense_infantry as f32) * upgrade_bonus * (stack.count as f32);
        dc += (unit.defense_cavalry as f32) * upgrade_bonus * (stack.count as f32);
    }
    (di, dc)
}

fn sigma(x: f32) -> f32 {
    if x > 1.0 {
        (2.0 - x.powf(-1.5)) / 2.0
    } else {
        x.powf(1.5) / 2.0
    }
}

fn calculate_ram_damage(
    surviving_rams: i32,
    attack_value: f32,
    defense_value: f32,
    wall_level: f32,
    wall_tribe: Tribe,
) -> f32 {
    if surviving_rams <= 0 || wall_level <= 0.0 {
        return wall_level;
    }

    // forza effettiva degli arieti
    let ram_attack_strength = surviving_rams as f32 * UNIT_RAM.attack_points as f32;

    // difesa del muro (scala con tribù e livello)
    let wall_strength_factor = match wall_tribe {
        Tribe::Romans => 2.0,
        Tribe::Teutons => 1.67,
        Tribe::Gauls => 1.75,
    };
    let wall_defense_value = wall_strength_factor * wall_level.powi(2);

    // confronto attacco/difesa
    let effective_attack = ram_attack_strength + attack_value;
    let effective_defense = defense_value + wall_defense_value;

    let ratio = effective_attack / effective_defense;
    if ratio <= 1.0 {
        return wall_level; // nessun danno
    }

    // danno proporzionale
    let wall_damage = (ratio.ln()) * 1.5; // coeff 1.5 ~ fedele al PHP

    // nuovo livello muro
    let new_wall_level = (wall_level - wall_damage).max(0.0);
    new_wall_level
}

fn calculate_catapult_damage(
    surviving_catas: i32,
    attack_value: f32,
    defense_value: f32,
    primary_target: BuildingId,
    secondary_target: Option<BuildingId>,
    building_levels: &mut HashMap<BuildingId, f32>,
) {
    if surviving_catas <= 0 {
        return;
    }

    // forza effettiva delle catapulte
    let catapult_attack_strength = surviving_catas as f32 * UNIT_CATAPULT.attack_points as f32;

    let effective_attack = catapult_attack_strength + attack_value;
    let effective_defense = defense_value.max(1.0);

    let ratio = effective_attack / effective_defense;
    if ratio <= 1.0 {
        return; // nessun danno
    }

    // danno base proporzionale al logaritmo del rapporto
    let base_damage = ratio.ln() * 1.5;

    // distribuzione: se due target, danno dimezzato per ciascuno
    let (damage_primary, damage_secondary) = if let Some(_) = secondary_target {
        (base_damage / 2.0, base_damage / 2.0)
    } else {
        (base_damage, 0.0)
    };

    // applica danno al building primario
    if let Some(level) = building_levels.get_mut(&primary_target) {
        *level = (*level - damage_primary).max(0.0);
    }

    // applica danno al building secondario
    if let Some(sec) = secondary_target {
        if let Some(level) = building_levels.get_mut(&sec) {
            *level = (*level - damage_secondary).max(0.0);
        }
    }
}

// -------------------------------------------------------------------------------------------

// src/battle_engine.rs

use std::collections::HashMap;

//================================================================================
// 1. PLACEHOLDER: STRUCT E DATI DI INPUT
// Queste sono le strutture che dovrai fornire con i dati reali del tuo gioco.
//================================================================================

// Enum per il tipo di battaglia, influenza il calcolo delle perdite.
pub enum BattleType {
    Normal,
    Raid,
    Scout,
}

// Enum per le tribù, influenza i bonus del muro.
pub enum Tribe {
    Romans,
    Teutons,
    Gauls,
    // ...altre
}

// Dati statici di una singola unità.
pub struct UnitData {
    pub attack: u32,
    pub inf_defense: u32,
    pub cav_defense: u32,
    pub is_cavalry: bool,
    pub is_scout: bool,
    pub population: u8,
    pub carry_capacity: u32,
}

// Rappresenta un esercito in una battaglia.
pub struct Army<'a> {
    pub tribe: &'a Tribe,
    pub units: HashMap<u32, u32>,            // K: unit_id, V: quantity
    pub blacksmith_levels: HashMap<u32, u8>, // K: unit_id, V: level
    pub hero_attack_bonus_percent: f64,      // es. 20.0 per +20%
    pub hero_defense_bonus_percent: f64,     // es. 20.0 per +20%
    pub hero_fighting_strength: u32,
    pub population: u32,
}

// Contiene tutti i dati necessari per un singolo calcolo di battaglia.
pub struct BattleInput<'a> {
    pub battle_type: BattleType,
    pub attacker: Army<'a>,
    // Il difensore può avere rinforzi, quindi è una lista di eserciti.
    pub reinforcements: Vec<Army<'a>>,
    pub wall_level: u8,
    pub residence_level: u8,
    pub stonemason_level: u8,
    pub target_building_level: u8, // Per le catapulte
}

// Il risultato della battaglia.
pub struct BattleReport {
    pub attacker_losses: HashMap<u32, u32>,
    pub defender_losses: HashMap<u32, u32>, // Qui aggregheremo le perdite di tutti i difensori
    pub new_wall_level: u8,
    pub new_target_building_level: u8,
    pub bounty: u32,
}

// --- Funzioni Placeholder (da implementare nel tuo codice) ---
fn get_unit_data(unit_id: u32) -> UnitData {
    // PLACEHOLDER:
    // Qui interroghi la tua struttura dati hard-coded per ottenere le statistiche
    // di una unità dato il suo ID.
    unimplemented!();
}

//================================================================================
// 2. MOTORE DI BATTAGLIA PRINCIPALE
//================================================================================

/// Funzione pubblica principale che funge da dispatcher.
pub fn calculate_battle(input: BattleInput) -> BattleReport {
    match input.battle_type {
        BattleType::Scout => calculate_scout_battle(&input),
        _ => calculate_combat_battle(&input),
    }
}

/// Calcola una battaglia di combattimento (Normale o Raid).
fn calculate_combat_battle(input: &BattleInput) -> BattleReport {
    // --- FASE 1: CALCOLO PUNTI BASE ---
    let (mut inf_attack_points, mut cav_attack_points) =
        calculate_base_attack_points(&input.attacker);

    // Applica bonus eroe all'attacco
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

/// Calcola la battaglia per gli esploratori (logica separata e più semplice).
fn calculate_scout_battle(input: &BattleInput) -> BattleReport {
    // PLACEHOLDER: La logica per gli scout è diversa.
    // 1. Calcola la "forza scout" dell'attaccante.
    // 2. Calcola la "forza difensiva scout" del difensore.
    // 3. Applica una formula simile ma più semplice per determinare le perdite.
    // 4. Se la forza attaccante è maggiore, il report non viene generato per il difensore.
    unimplemented!();
}

//================================================================================
// 3. FUNZIONI HELPER DI CALCOLO
//================================================================================

/// Calcola i punti di attacco base per un esercito, applicando il bonus del fabbro.
fn calculate_base_attack_points(army: &Army) -> (f64, f64) {
    let mut inf_points = 0.0;
    let mut cav_points = 0.0;

    for (unit_id, quantity) in &army.units {
        let data = get_unit_data(*unit_id);
        if data.attack == 0 {
            continue;
        }

        let blacksmith_level = army.blacksmith_levels.get(unit_id).cloned().unwrap_or(0);
        let bonus = (data.attack as f64
            + (data.attack as f64 + 300.0 * data.population as f64 / 7.0)
                * ((1.007_f64).powi(blacksmith_level as i32) - 1.0));
        let total_attack = bonus * (*quantity as f64);

        if data.is_cavalry {
            cav_points += total_attack;
        } else {
            inf_points += total_attack;
        }
    }
    (inf_points, cav_points)
}

/// Calcola i punti di difesa base per un esercito, applicando il bonus dell'armeria.
fn calculate_base_defense_points(army: &Army) -> (f64, f64) {
    let mut inf_points = 0.0;
    let mut cav_points = 0.0;

    // La logica è identica a quella dell'attacco, ma usando i valori di difesa.
    // PLACEHOLDER: Implementare la logica speculare a `calculate_base_attack_points`
    // usando `data.inf_defense`, `data.cav_defense` e l'armeria invece del fabbro.

    (inf_points, cav_points)
}

/// Calcola il moltiplicatore di difesa dato dal muro.
fn calculate_wall_bonus(tribe: &Tribe, level: u8) -> f64 {
    if level == 0 {
        return 1.0;
    }
    let factor = match tribe {
        Tribe::Romans => 1.030,
        Tribe::Teutons => 1.020,
        Tribe::Gauls => 1.025,
        _ => 1.020, // Default
    };
    factor.powi(level as i32)
}

/// Calcola il moltiplicatore di morale (malus per l'attaccante se ha molta più popolazione).
fn calculate_morale_bonus(attacker_pop: u32, defender_pop: u32) -> f64 {
    if attacker_pop <= defender_pop {
        return 1.0;
    }
    let pop_ratio = defender_pop as f64 / attacker_pop as f64;
    // La formula nel PHP è più complessa ma il nucleo è questo.
    // Per fedeltà, usiamo quella: `max(0.667, (pop_ratio)^0.2)`
    pop_ratio.powf(0.2).max(0.667)
}

/// L'esponente della formula di battaglia non è sempre 1.5.
/// Dipende dal numero totale di unità coinvolte.
fn calculate_m_factor(total_troops: u32) -> f64 {
    if total_troops < 1000 {
        1.5
    } else {
        // Questa è la formula esatta trovata nel codice PHP.
        let factor = 2.0 * (1.8592 - (total_troops as f64).powf(0.015));
        factor.clamp(1.2578, 1.5) // Limita il valore tra un minimo e un massimo.
    }
}

/// Funzione di utilità per generare un report vuoto in caso di attacco nullo.
fn generate_empty_report(input: &BattleInput) -> BattleReport {
    // ...
    unimplemented!();
}
