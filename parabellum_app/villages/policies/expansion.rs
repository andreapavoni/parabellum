use parabellum_game::models::village::Village;
use parabellum_types::{
    army::{UnitName, UnitRole},
    common::Speed,
    errors::GameError,
};
use uuid::Uuid;

use crate::ports::queries::{TrainingQueueItem, VillageTroopMovements};

/// Expansion slot usage for a village.
///
/// This is the policy boundary for chief/settler availability. Aggregate-local
/// validation builds it from the village state and pending training actions;
/// read-side validation can also include already-founded child villages and
/// moving expansion units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpansionSlotUsage {
    pub max_slots: u8,
    pub child_villages: u8,
    pub chiefs_at_home: u32,
    pub settlers_at_home: u32,
    pub chiefs_deployed: u32,
    pub settlers_deployed: u32,
    pub chiefs_queued: u32,
    pub settlers_queued: u32,
    pub chiefs_moving: u32,
    pub settlers_moving: u32,
}

/// A queued expansion-unit training commitment.
///
/// This intentionally uses the shared unit name and remaining quantity instead
/// of a scheduled-action payload. Both aggregate state and read-model queue DTOs
/// can be converted into this shape without exposing their storage details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpansionTrainingCommitment {
    /// Queued unit type.
    pub unit: UnitName,
    /// Remaining units in the queued training action.
    pub quantity_remaining: i32,
}

/// Battle-time conquest eligibility.
///
/// This policy intentionally covers only whether a conquest may be attempted at
/// battle resolution time. The battle domain still owns loyalty damage and the
/// final conquest outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConquestAttempt {
    pub target_is_capital: bool,
    pub attacking_chiefs: u32,
    pub source_max_slots: u8,
    pub source_child_villages: u8,
    pub player_village_count: usize,
    pub player_culture_points: u32,
    pub speed: Speed,
}

impl ConquestAttempt {
    pub fn is_allowed(&self) -> bool {
        if self.target_is_capital {
            return false;
        }
        if self.attacking_chiefs == 0 {
            return false;
        }
        if self.source_max_slots == 0 {
            return false;
        }
        if self.source_child_villages >= self.source_max_slots {
            return false;
        }

        self.player_culture_points
            >= parabellum_game::models::culture_points::required_cp(
                self.speed,
                self.player_village_count + 1,
            )
    }
}

impl ExpansionSlotUsage {
    pub fn from_village_context(
        village: &Village,
        child_villages: u8,
        training_queue: &[TrainingQueueItem],
        movements: &VillageTroopMovements,
        player_id: Uuid,
    ) -> Self {
        let commitments = training_queue
            .iter()
            .map(|item| ExpansionTrainingCommitment {
                unit: item.unit.clone(),
                quantity_remaining: item.quantity,
            })
            .collect::<Vec<_>>();
        Self::from_parts(
            village,
            child_villages,
            &commitments,
            moving_expansion_units(movements, player_id),
        )
    }

    pub fn from_local_village(
        village: &Village,
        training_commitments: &[ExpansionTrainingCommitment],
    ) -> Self {
        Self::from_parts(village, 0, training_commitments, (0, 0))
    }

    fn from_parts(
        village: &Village,
        child_villages: u8,
        training_commitments: &[ExpansionTrainingCommitment],
        moving_units: (u32, u32),
    ) -> Self {
        let (chiefs_queued, settlers_queued) = queued_expansion_units(training_commitments);
        let (chiefs_moving, settlers_moving) = moving_units;

        Self {
            max_slots: village.max_foundation_slots(),
            child_villages,
            chiefs_at_home: village.count_chiefs_at_home(),
            settlers_at_home: village.count_settlers_at_home(),
            chiefs_deployed: village
                .deployed_armies()
                .iter()
                .map(|army| army.units().get(8))
                .sum(),
            settlers_deployed: village
                .deployed_armies()
                .iter()
                .map(|army| army.units().get(9))
                .sum(),
            chiefs_queued,
            settlers_queued,
            chiefs_moving,
            settlers_moving,
        }
    }

    pub fn free_slots(&self) -> u8 {
        self.max_slots.saturating_sub(self.child_villages)
    }

    pub fn chiefs_total(&self) -> u32 {
        self.chiefs_at_home + self.chiefs_deployed + self.chiefs_queued + self.chiefs_moving
    }

    pub fn settlers_total(&self) -> u32 {
        self.settlers_at_home + self.settlers_deployed + self.settlers_queued + self.settlers_moving
    }

    pub fn max_trainable(&self, unit_role: UnitRole) -> u32 {
        let chiefs_total = self.chiefs_total();
        let settlers_total = self.settlers_total();
        let committed_this_unit = match unit_role {
            UnitRole::Chief => chiefs_total,
            UnitRole::Settler => settlers_total,
            _ => 0,
        };

        Village::max_expansion_unit_trainable(
            unit_role,
            self.free_slots(),
            chiefs_total,
            settlers_total,
            committed_this_unit,
        )
    }

    pub fn validate_training(&self, unit_role: UnitRole, quantity: i32) -> Result<(), GameError> {
        if !unit_role.is_expansion() {
            return Ok(());
        }
        if self.free_slots() == 0 {
            return Err(GameError::NoFoundationSlotsAvailable);
        }

        let requested = quantity as u32;
        let max_trainable = self.max_trainable(unit_role);
        if requested <= max_trainable {
            return Ok(());
        }

        match unit_role {
            UnitRole::Chief => Err(GameError::ChiefLimitExceeded {
                max: max_trainable,
                current: self.chiefs_total(),
                requested,
            }),
            UnitRole::Settler => Err(GameError::SettlerLimitExceeded {
                max: max_trainable + self.settlers_total(),
                current: self.settlers_total(),
                requested,
            }),
            _ => Ok(()),
        }
    }
}

fn queued_expansion_units(training_queue: &[ExpansionTrainingCommitment]) -> (u32, u32) {
    let mut chiefs = 0u32;
    let mut settlers = 0u32;
    for item in training_queue {
        let quantity = item.quantity_remaining.max(0) as u32;
        if matches!(
            item.unit,
            UnitName::Chief | UnitName::Senator | UnitName::Chieftain
        ) {
            chiefs = chiefs.saturating_add(quantity);
        } else if matches!(item.unit, UnitName::Settler) {
            settlers = settlers.saturating_add(quantity);
        }
    }
    (chiefs, settlers)
}

fn moving_expansion_units(movements: &VillageTroopMovements, player_id: Uuid) -> (u32, u32) {
    movements
        .outgoing
        .iter()
        .chain(movements.incoming.iter())
        .filter(|movement| movement.origin_player_id == player_id)
        .fold((0u32, 0u32), |(chiefs, settlers), movement| {
            (
                chiefs.saturating_add(movement.units.get(8)),
                settlers.saturating_add(movement.units.get(9)),
            )
        })
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use parabellum_types::{
        army::{TroopSet, UnitName, UnitRole},
        common::ResourceGroup,
        map::Position,
        tribe::Tribe,
    };

    use super::*;
    use crate::ports::queries::{TroopMovement, TroopMovementDirection};

    fn usage(max_slots: u8, child_villages: u8) -> ExpansionSlotUsage {
        ExpansionSlotUsage {
            max_slots,
            child_villages,
            chiefs_at_home: 0,
            settlers_at_home: 0,
            chiefs_deployed: 0,
            settlers_deployed: 0,
            chiefs_queued: 0,
            settlers_queued: 0,
            chiefs_moving: 0,
            settlers_moving: 0,
        }
    }

    #[test]
    fn validates_available_settler_capacity() {
        let usage = usage(1, 0);

        assert_eq!(usage.max_trainable(UnitRole::Settler), 3);
        assert!(usage.validate_training(UnitRole::Settler, 3).is_ok());
    }

    #[test]
    fn allows_conquest_attempt_when_chief_slot_and_culture_points_are_available() {
        let attempt = ConquestAttempt {
            target_is_capital: false,
            attacking_chiefs: 1,
            source_max_slots: 1,
            source_child_villages: 0,
            player_village_count: 1,
            player_culture_points: parabellum_game::models::culture_points::required_cp(
                Speed::X1,
                2,
            ),
            speed: Speed::X1,
        };

        assert!(attempt.is_allowed());
    }

    #[test]
    fn rejects_conquest_attempt_without_free_source_slot() {
        let attempt = ConquestAttempt {
            target_is_capital: false,
            attacking_chiefs: 1,
            source_max_slots: 1,
            source_child_villages: 1,
            player_village_count: 1,
            player_culture_points: u32::MAX,
            speed: Speed::X1,
        };

        assert!(!attempt.is_allowed());
    }

    #[test]
    fn rejects_conquest_attempt_against_capital_or_without_chiefs() {
        let base = ConquestAttempt {
            target_is_capital: false,
            attacking_chiefs: 1,
            source_max_slots: 1,
            source_child_villages: 0,
            player_village_count: 1,
            player_culture_points: u32::MAX,
            speed: Speed::X1,
        };

        assert!(
            !ConquestAttempt {
                target_is_capital: true,
                ..base.clone()
            }
            .is_allowed()
        );
        assert!(
            !ConquestAttempt {
                attacking_chiefs: 0,
                ..base
            }
            .is_allowed()
        );
    }

    #[test]
    fn returns_no_slots_when_children_fill_capacity() {
        let usage = usage(1, 1);

        assert!(matches!(
            usage.validate_training(UnitRole::Settler, 1),
            Err(GameError::NoFoundationSlotsAvailable)
        ));
    }

    #[test]
    fn returns_chief_limit_error_when_chiefs_exceed_remaining_capacity() {
        let mut usage = usage(2, 0);
        usage.settlers_queued = 3;

        assert_eq!(usage.max_trainable(UnitRole::Chief), 1);
        assert!(matches!(
            usage.validate_training(UnitRole::Chief, 2),
            Err(GameError::ChiefLimitExceeded {
                max: 1,
                current: 0,
                requested: 2,
            })
        ));
    }

    #[test]
    fn returns_settler_limit_error_with_current_and_max_totals() {
        let mut usage = usage(2, 0);
        usage.chiefs_moving = 1;
        usage.settlers_at_home = 1;
        usage.settlers_queued = 1;

        assert_eq!(usage.max_trainable(UnitRole::Settler), 1);
        assert!(matches!(
            usage.validate_training(UnitRole::Settler, 2),
            Err(GameError::SettlerLimitExceeded {
                max: 3,
                current: 2,
                requested: 2,
            })
        ));
    }

    #[test]
    fn counts_queued_and_moving_expansion_units() {
        let player_id = Uuid::new_v4();
        let other_player_id = Uuid::new_v4();
        let movements = VillageTroopMovements {
            outgoing: vec![movement(player_id, 1, 2)],
            incoming: vec![movement(other_player_id, 10, 10)],
        };
        let commitments = vec![
            ExpansionTrainingCommitment {
                unit: UnitName::Senator,
                quantity_remaining: 1,
            },
            ExpansionTrainingCommitment {
                unit: UnitName::Settler,
                quantity_remaining: 2,
            },
        ];

        assert_eq!(queued_expansion_units(&commitments), (1, 2));
        assert_eq!(moving_expansion_units(&movements, player_id), (1, 2));
    }

    fn movement(player_id: Uuid, chiefs: u32, settlers: u32) -> TroopMovement {
        let mut units = TroopSet::default();
        units.set(8, chiefs);
        units.set(9, settlers);
        TroopMovement {
            job_id: Uuid::new_v4(),
            movement_type: crate::ports::queries::TroopMovementType::Attack,
            direction: TroopMovementDirection::Outgoing,
            origin_village_id: 1,
            origin_village_name: None,
            origin_player_id: player_id,
            origin_position: Position { x: 0, y: 0 },
            target_village_id: 2,
            target_village_name: None,
            target_player_id: player_id,
            target_position: Position { x: 1, y: 1 },
            arrives_at: Utc::now(),
            time_seconds: 10,
            units,
            has_hero: false,
            tribe: Tribe::Roman,
            bounty: Some(ResourceGroup::default()),
        }
    }
}
