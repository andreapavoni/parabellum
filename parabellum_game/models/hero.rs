use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Hero {
    pub id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub health: u16,
    pub experience: u32,
    pub attack_points: u32,
    pub defense_points: u32,
    pub off_bonus: u16,
    pub def_bonus: u16,
}

impl Hero {
    /// Returns a new hero instance.
    pub fn new(id: Option<Uuid>, village_id: u32, player_id: Uuid) -> Self {
        Hero {
            id: id.unwrap_or(Uuid::new_v4()),
            village_id: village_id,
            player_id,
            health: 100,
            experience: 0,
            attack_points: 0,
            defense_points: 0,
            off_bonus: 0,
            def_bonus: 0,
        }
    }

    /// Returns the total attack bonus.
    pub fn get_attack_bonus(&self, is_attacking_with_army: bool) -> u32 {
        if is_attacking_with_army {
            let base_attack = 100; // FXIME: fix this
            (base_attack as f64 * (1.0 + Self::get_bonus_by_points(self.attack_points))) as u32
        } else {
            0
        }
    }

    /// Returns the total defense bonus (fixed points).
    pub fn get_defense_bonus(&self) -> u32 {
        let base_defense = 100; // FIXME: change this value
        (base_defense as f64 * (1.0 + Self::get_bonus_by_points(self.defense_points))) as u32
    }

    /// Calculate bonus by skill points.
    /// Each point gives a 0.8% bonus, compounded.
    fn get_bonus_by_points(points: u32) -> f64 {
        if points == 0 {
            0.0
        } else {
            1.008f64.powi(points as i32) - 1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bonus_by_points() {
        // 0 points = 0.0 bonus
        assert_eq!(Hero::get_bonus_by_points(0), 0.0);

        // 1 point = 1.008^1 - 1 = 0.008
        let bonus_1 = Hero::get_bonus_by_points(1);
        // Use an epsilon for float comparison
        const EPSILON: f64 = 1e-9;
        assert!(
            (bonus_1 - 0.008).abs() < EPSILON,
            "Bonus was {}, expected ~0.008",
            bonus_1
        );

        // 10 points = 1.008^10 - 1 = ~0.0828
        let bonus_10 = Hero::get_bonus_by_points(10);
        assert!(
            bonus_10 > 0.0828 && bonus_10 < 0.083,
            "Bonus was {}, expected ~0.0829",
            bonus_10
        );
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
