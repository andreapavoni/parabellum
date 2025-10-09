use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Hero {
    pub player_id: Uuid,
    pub health: u16,
    pub experience: u32,
    /// Punti abilità investiti in attacco.
    pub attack_points: u32,
    /// Punti abilità investiti in difesa.
    pub defense_points: u32,
    /// Bonus offensivo percentuale (es. 0.1 per 10%).
    pub off_bonus: u16,
    /// Bonus difensivo percentuale (es. 0.1 per 10%).
    pub def_bonus: u16,
}

impl Hero {
    /// Calcola il bonus di combattimento basato sui punti abilità.
    /// Ogni punto abilità aumenta la statistica dello 0.8% (elevato alla potenza del numero di punti).
    fn get_bonus_by_points(points: u32) -> f64 {
        if points == 0 {
            0.0
        } else {
            1.008f64.powi(points as i32) - 1.0
        }
    }

    /// Ritorna il bonus offensivo totale (punti fissi).
    /// Si applica solo se l'eroe attacca con l'armata.
    pub fn get_attack_bonus(&self, is_attacking_with_army: bool) -> u32 {
        if is_attacking_with_army {
            // L'eroe ha una base di attacco che potrebbe essere definita qui o presa da dati statici
            let base_attack = 100; // Esempio
            (base_attack as f64 * (1.0 + Self::get_bonus_by_points(self.attack_points))) as u32
        } else {
            0
        }
    }

    /// Ritorna il bonus difensivo totale (punti fissi).
    pub fn get_defense_bonus(&self) -> u32 {
        let base_defense = 100; // Esempio
        (base_defense as f64 * (1.0 + Self::get_bonus_by_points(self.defense_points))) as u32
    }
}
