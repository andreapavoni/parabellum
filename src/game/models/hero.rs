use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Hero {
    pub id: Uuid,
    pub player_id: Uuid,
    pub health: u16,
    pub experience: u32,
    /// Attack skill points.
    pub attack_points: u32,
    /// Defense skill points.
    pub defense_points: u32,
    /// Attack bonus percent (ex. 0.1 for 10%).
    pub off_bonus: u16,
    /// Defense bonus percent (ex. 0.1 for 10%).
    pub def_bonus: u16,
}

impl Hero {
    /// Calculate bonus by skill points.
    /// Each point gives a 0.8% bonus, compounded.
    fn get_bonus_by_points(points: u32) -> f64 {
        if points == 0 {
            0.0
        } else {
            1.008f64.powi(points as i32) - 1.0
        }
    }

    /// Returns the total attack bonus (fixed points).
    pub fn get_attack_bonus(&self, is_attacking_with_army: bool) -> u32 {
        if is_attacking_with_army {
            let base_attack = 100; // TODO: fix this
            (base_attack as f64 * (1.0 + Self::get_bonus_by_points(self.attack_points))) as u32
        } else {
            0
        }
    }

    /// Returns the total defense bonus (fixed points).
    pub fn get_defense_bonus(&self) -> u32 {
        let base_defense = 100; // Esempio
        (base_defense as f64 * (1.0 + Self::get_bonus_by_points(self.defense_points))) as u32
    }
}

// --- MODULO TEST AGGIUNTO ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bonus_by_points() {
        // 0 points = 0.0 bonus
        assert_eq!(Hero::get_bonus_by_points(0), 0.0);

        // 1 point = 1.008^1 - 1 = 0.008
        assert_eq!(Hero::get_bonus_by_points(1), 0.008);

        // 10 points = 1.008^10 - 1 = ~0.0828
        let bonus_10 = Hero::get_bonus_by_points(10);
        assert!(bonus_10 > 0.0828 && bonus_10 < 0.0829);
    }

    #[test]
    fn test_get_attack_bonus() {
        let mut hero = Hero::default();
        hero.attack_points = 10;

        // Not with army
        assert_eq!(hero.get_attack_bonus(false), 0);

        // With army (base 100 + ~8.28% bonus)
        let attack = hero.get_attack_bonus(true);
        assert_eq!(attack, 108); // 100.0 * (1.0 + 0.0828...) = 108.28... -> 108
    }

    #[test]
    fn test_get_defense_bonus() {
        let mut hero = Hero::default();
        hero.defense_points = 1;

        // With army (base 100 + 0.8% bonus)
        let defense = hero.get_defense_bonus();
        assert_eq!(defense, 100); // 100.0 * (1.0 + 0.008) = 100.8 -> 100
    }
}
