//! Village activity read models.
//!
//! Activity views summarize scheduled village work and troop movement state for
//! application/UI reads. They deliberately stay separate from command payloads
//! and persistence projection rows.

use chrono::{DateTime, Utc};
use parabellum_types::{
    army::{TroopSet, UnitName},
    buildings::BuildingName,
    common::ResourceGroup,
    map::Position,
};
use uuid::Uuid;

use crate::villages::models::{BuildingWorkflowKind, ScheduledActionStatus};

/// A pending building construction, upgrade, downgrade, or cancellation target.
#[derive(Debug, Clone)]
pub struct BuildingQueueItem {
    pub job_id: Uuid,
    pub kind: BuildingWorkflowKind,
    pub slot_id: u8,
    pub building_name: BuildingName,
    pub target_level: u8,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

/// A pending unit-training queue item.
#[derive(Debug, Clone)]
pub struct TrainingQueueItem {
    pub job_id: Uuid,
    pub slot_id: u8,
    pub unit: UnitName,
    pub quantity: i32,
    pub time_per_unit: i32,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

/// A pending academy research queue item.
#[derive(Debug, Clone)]
pub struct AcademyQueueItem {
    pub job_id: Uuid,
    pub unit: UnitName,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

/// A pending smithy upgrade queue item.
#[derive(Debug, Clone)]
pub struct SmithyQueueItem {
    pub job_id: Uuid,
    pub unit: UnitName,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

/// A pending trap-building queue item.
#[derive(Debug, Clone)]
pub struct TrapQueueItem {
    pub job_id: Uuid,
    pub quantity: i32,
    pub time_per_trap: i32,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

/// Queue summary for all scheduled village work shown in village activity UI.
#[derive(Debug, Clone, Default)]
pub struct VillageQueues {
    pub building: Vec<BuildingQueueItem>,
    pub training: Vec<TrainingQueueItem>,
    pub academy: Vec<AcademyQueueItem>,
    pub smithy: Vec<SmithyQueueItem>,
    pub traps: Vec<TrapQueueItem>,
}

/// App-facing troop movement category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TroopMovementType {
    Attack,
    Raid,
    Scout,
    Reinforcement,
    Return,
    FoundVillage,
}

/// Whether a troop movement is arriving at or leaving the selected village.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TroopMovementDirection {
    Incoming,
    Outgoing,
}

/// App-facing troop movement summary.
#[derive(Debug, Clone, PartialEq)]
pub struct TroopMovement {
    pub job_id: Uuid,
    pub movement_type: TroopMovementType,
    pub direction: TroopMovementDirection,
    pub origin_village_id: u32,
    pub origin_village_name: Option<String>,
    pub origin_player_id: Uuid,
    pub origin_position: Position,
    pub target_village_id: u32,
    pub target_village_name: Option<String>,
    pub target_player_id: Uuid,
    pub target_position: Position,
    pub arrives_at: DateTime<Utc>,
    pub time_seconds: u32,
    pub units: TroopSet,
    pub has_hero: bool,
    pub tribe: parabellum_types::tribe::Tribe,
    pub bounty: Option<ResourceGroup>,
}

impl TroopMovement {
    /// Returns whether this row should expose army composition to the viewing
    /// village.
    ///
    /// Hostile incoming attacks, raids, and scouts are intentionally opaque
    /// until battle reports exist. Friendly incoming reinforcements, returns,
    /// and founding movements may expose their composition because they are
    /// visible from the owner's perspective.
    pub fn exposes_army_composition_to_viewer(&self) -> bool {
        match self.direction {
            TroopMovementDirection::Outgoing => true,
            TroopMovementDirection::Incoming => matches!(
                self.movement_type,
                TroopMovementType::Reinforcement
                    | TroopMovementType::Return
                    | TroopMovementType::FoundVillage
            ),
        }
    }
}

/// Incoming and outgoing troop movement summary for a village.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VillageTroopMovements {
    pub outgoing: Vec<TroopMovement>,
    pub incoming: Vec<TroopMovement>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use parabellum_types::{map::Position, tribe::Tribe};

    fn movement(
        direction: TroopMovementDirection,
        movement_type: TroopMovementType,
    ) -> TroopMovement {
        TroopMovement {
            job_id: Uuid::nil(),
            movement_type,
            direction,
            origin_village_id: 1,
            origin_village_name: None,
            origin_player_id: Uuid::nil(),
            origin_position: Position { x: 0, y: 0 },
            target_village_id: 2,
            target_village_name: None,
            target_player_id: Uuid::nil(),
            target_position: Position { x: 1, y: 1 },
            arrives_at: DateTime::<Utc>::UNIX_EPOCH,
            time_seconds: 0,
            units: TroopSet::default(),
            has_hero: false,
            tribe: Tribe::Roman,
            bounty: None,
        }
    }

    #[test]
    fn outgoing_movements_expose_composition_to_owner() {
        let movement = movement(TroopMovementDirection::Outgoing, TroopMovementType::Attack);

        assert!(movement.exposes_army_composition_to_viewer());
    }

    #[test]
    fn hostile_incoming_movements_hide_composition_from_target() {
        for movement_type in [
            TroopMovementType::Attack,
            TroopMovementType::Raid,
            TroopMovementType::Scout,
        ] {
            let movement = movement(TroopMovementDirection::Incoming, movement_type);

            assert!(!movement.exposes_army_composition_to_viewer());
        }
    }

    #[test]
    fn friendly_incoming_movements_expose_composition_to_viewer() {
        for movement_type in [
            TroopMovementType::Reinforcement,
            TroopMovementType::Return,
            TroopMovementType::FoundVillage,
        ] {
            let movement = movement(TroopMovementDirection::Incoming, movement_type);

            assert!(movement.exposes_army_composition_to_viewer());
        }
    }
}
